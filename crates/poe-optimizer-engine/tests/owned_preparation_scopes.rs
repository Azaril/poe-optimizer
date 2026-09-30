//! Static scope authority; concrete assignment/skill binding is tested in plans.
#[allow(dead_code)]
#[path = "support/owned_rule_fixture.rs"]
mod fixture;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_engine::owned_rules::*;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn id<K: DefinitionDomain>(value: &str) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("authored-rule-tests", "v1").unwrap(),
        value,
    )
    .unwrap()
}
fn subject<K: SchemaDefinitionId>(id: K) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
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
fn base() -> (OwnedDefinitionSchemaPackage, RulePackageInput) {
    let original = fixture::fixture();
    let mut raw = original.schema.input().clone();
    raw.schema_version = OWNED_SCHEMA_PACKAGE_V4;
    for (name, target) in [
        ("origin-value", RuleEntityKind::SupportOrigin),
        ("skill-value", RuleEntityKind::Skill),
    ] {
        raw.definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: id(name),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![target],
                }),
            }));
    }
    raw.definitions
        .push(DefinitionDescriptor::Capability(DefinitionEntry {
            id: id("origin-capability"),
            schema: SchemaState::Known(CapabilitySchema {
                targets: vec![RuleEntityKind::SupportOrigin],
            }),
        }));
    raw.definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: id("skill.target"),
            schema: SchemaState::Known(SkillSchema {
                directly_selectable: true,
                declarations: declarations(),
            }),
        }));
    let schema = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let mut rules = original.rules;
    rules.definitions = schema.identity().clone();
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V12);
    rules.owners = vec![DefinitionRules {
        owner: subject(id::<GemDefinition>("gem.support")),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("prepare"),
            context: RuleEntityKind::SupportOrigin,
            reads: vec![RuleRead {
                id: key("level"),
                value_type: ComputedValueType::Integer,
                source: RuleReadSource::GemLevel,
            }],
            nodes: vec![
                RuleNode {
                    id: key("level-value"),
                    expression: RuleExpression::Read {
                        input: key("level"),
                    },
                },
                RuleNode {
                    id: key("yes"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    },
                },
            ],
            effects: vec![RuleEffect {
                id: key("own-level"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: id("origin-value"),
                    value: key("level-value"),
                },
            }],
        }]),
    }];
    (schema, rules)
}
fn program(rules: &mut RulePackageInput) -> &mut RuleProgram {
    &mut rules.owners[0].programs.members[0]
}
fn compile(
    schema: &OwnedDefinitionSchemaPackage,
    rules: &RulePackageInput,
) -> Result<CompiledRulePackage, RuleError> {
    CompiledRulePackage::compile(rules, schema, RuleLimits::default())
}
fn fails(schema: &OwnedDefinitionSchemaPackage, rules: &RulePackageInput, expected: &str) {
    let error = compile(schema, rules).unwrap_err();
    assert!(error.to_string().contains(expected), "{error}");
}
fn gem_roles(
    schema: &OwnedDefinitionSchemaPackage,
    rules: &mut RulePackageInput,
    roles: Vec<AuthoredGemRole>,
) -> OwnedDefinitionSchemaPackage {
    let mut raw = schema.input().clone();
    for descriptor in &mut raw.definitions {
        if let DefinitionDescriptor::Gem(DefinitionEntry {
            schema: SchemaState::Known(gem),
            ..
        }) = descriptor
        {
            gem.roles = roles.clone();
        }
    }
    let changed = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    rules.definitions = changed.identity().clone();
    changed
}

#[test]
fn origin_programs_compute_own_values_capabilities_contributions_and_requirements() {
    let (schema, mut rules) = base();
    let p = program(&mut rules);
    p.effects.extend([
        RuleEffect {
            id: key("capability"),
            when: None,
            effect: RuleEffectKind::Capability {
                entity: RuleEntity::SupportOrigin,
                capability: id("origin-capability"),
                enabled: key("yes"),
            },
        },
        RuleEffect {
            id: key("contribution"),
            when: None,
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::SupportOrigin,
                stat: id("origin-value"),
                contribution: ContributionKind::Add,
                value: key("level-value"),
            },
        },
        RuleEffect {
            id: key("requirement"),
            when: None,
            effect: RuleEffectKind::Requirement {
                satisfied: key("yes"),
                code: key("declared"),
            },
        },
    ]);
    let compiled = compile(&schema, &rules).unwrap();
    let output = compiled
        .evaluate(
            &rules.owners[0].owner,
            &key("prepare"),
            &[RuleFact {
                read: key("level"),
                value: ParameterValue::Integer(BoundedInteger::new(7).unwrap()),
            }],
            &schema,
            &mut compiled.new_scratch(),
        )
        .unwrap();
    assert_eq!(output.effects.len(), 4);
    for version in [
        OWNED_RULE_OPERATIONS_V6,
        OWNED_RULE_OPERATIONS_V7,
        OWNED_RULE_OPERATIONS_V8,
        OWNED_RULE_OPERATIONS_V9,
        OWNED_RULE_OPERATIONS_V10,
        OWNED_RULE_OPERATIONS_V11,
    ] {
        rules.operations_version = key(version);
        fails(
            &schema,
            &rules,
            "preparation scopes require owned-domain-operations-v12",
        );
    }
}

#[test]
fn relative_scope_aliases_have_explicit_contexts_and_do_not_infer_receiver_actors() {
    for (context, entity, allowed) in [
        (RuleEntityKind::SupportOrigin, RuleEntity::Current, true),
        (
            RuleEntityKind::SupportOrigin,
            RuleEntity::SupportOrigin,
            true,
        ),
        (
            RuleEntityKind::SupportOrigin,
            RuleEntity::AssignedSkill,
            true,
        ),
        (RuleEntityKind::SupportOrigin, RuleEntity::Player, true),
        (RuleEntityKind::SupportOrigin, RuleEntity::Actor, false),
        (RuleEntityKind::SupportOrigin, RuleEntity::Skill, false),
        (RuleEntityKind::Actor, RuleEntity::SupportOrigin, true),
        (RuleEntityKind::Actor, RuleEntity::AssignedSkill, true),
        (RuleEntityKind::Actor, RuleEntity::Skill, false),
        (RuleEntityKind::Action, RuleEntity::SupportOrigin, true),
        (RuleEntityKind::Action, RuleEntity::AssignedSkill, true),
        (RuleEntityKind::Action, RuleEntity::Skill, true),
        (
            RuleEntityKind::EquipmentUse,
            RuleEntity::SupportOrigin,
            false,
        ),
        (
            RuleEntityKind::EquipmentUse,
            RuleEntity::AssignedSkill,
            false,
        ),
    ] {
        let (schema, mut rules) = base();
        let p = program(&mut rules);
        p.context = context;
        p.effects.clear();
        p.reads[0].source = RuleReadSource::Stat {
            entity,
            stat: id(match entity {
                RuleEntity::Skill | RuleEntity::AssignedSkill => "skill-value",
                RuleEntity::Player | RuleEntity::Actor => "stat.effective-attribute",
                _ => "origin-value",
            }),
        };
        let result = compile(&schema, &rules);
        assert_eq!(
            result.is_ok(),
            allowed,
            "{context:?}/{entity:?}: {result:?}"
        );
        if allowed {
            rules.operations_version = key(OWNED_RULE_OPERATIONS_V11);
            fails(
                &schema,
                &rules,
                "preparation scopes require owned-domain-operations-v12",
            );
        }
    }
}

#[test]
fn origin_scope_is_authorized_by_support_gem_role_not_actor_or_action_context_alone() {
    for context in [
        RuleEntityKind::SupportOrigin,
        RuleEntityKind::Actor,
        RuleEntityKind::Action,
    ] {
        for entity in [RuleEntity::SupportOrigin, RuleEntity::AssignedSkill] {
            let (schema, mut rules) = base();
            let p = program(&mut rules);
            p.context = context;
            p.effects.clear();
            p.reads[0].source = RuleReadSource::Stat {
                entity,
                stat: id(if entity == RuleEntity::AssignedSkill {
                    "skill-value"
                } else {
                    "origin-value"
                }),
            };
            let wrong_role = gem_roles(&schema, &mut rules, vec![AuthoredGemRole::SkillUse]);
            fails(&wrong_role, &rules, "SupportAssignment Gem owner");
            rules.definitions = schema.identity().clone();
            for owner in [
                subject(id::<ItemTemplateDefinition>("item.template")),
                subject(id::<ModifierDefinition>("modifier.conditioned")),
                subject(id::<SkillDefinition>("skill.target")),
            ] {
                rules.owners[0].owner = owner;
                fails(&schema, &rules, "SupportAssignment Gem owner");
            }
        }
    }
    // Even a context-only origin rule with no explicit alias needs the Gem role.
    let (schema, mut rules) = base();
    program(&mut rules).reads.clear();
    program(&mut rules).nodes.clear();
    program(&mut rules).effects.clear();
    let wrong_role = gem_roles(&schema, &mut rules, vec![AuthoredGemRole::SkillUse]);
    fails(&wrong_role, &rules, "SupportAssignment Gem owner");
}

#[test]
fn skill_context_requires_skill_or_active_gem_and_keeps_support_assignment_alias_distinct() {
    let (schema, mut rules) = base();
    let p = program(&mut rules);
    p.context = RuleEntityKind::Skill;
    p.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Skill,
        stat: id("skill-value"),
    };
    p.effects[0].effect = RuleEffectKind::Derive {
        entity: RuleEntity::Current,
        stat: id("skill-value"),
        value: key("level-value"),
    };
    fails(
        &schema,
        &rules,
        "Skill context requires a Skill or SkillUse Gem owner",
    );
    let active = gem_roles(&schema, &mut rules, vec![AuthoredGemRole::SkillUse]);
    compile(&active, &rules).unwrap();
    rules.owners[0].owner = subject(id::<SkillDefinition>("skill.target"));
    compile(&active, &rules).unwrap();
    program(&mut rules).reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::AssignedSkill,
        stat: id("skill-value"),
    };
    fails(&active, &rules, "SupportAssignment Gem owner");
    rules.owners[0].owner = subject(id::<GemDefinition>("gem.support"));
    let dual = gem_roles(
        &active,
        &mut rules,
        vec![
            AuthoredGemRole::SkillUse,
            AuthoredGemRole::SupportAssignment,
        ],
    );
    fails(&dual, &rules, "relative support scope requires");
}

#[test]
fn origin_context_cannot_write_target_or_global_values_or_trigger_delivery() {
    let (schema, rules) = base();
    let grant = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
        slot: id("grant.actor"),
    };
    for effect in [
        RuleEffectKind::Derive {
            entity: RuleEntity::Player,
            stat: id("stat.effective-attribute"),
            value: key("level-value"),
        },
        RuleEffectKind::Contribute {
            entity: RuleEntity::AssignedSkill,
            stat: id("skill-value"),
            contribution: ContributionKind::Add,
            value: key("level-value"),
        },
        RuleEffectKind::Capability {
            entity: RuleEntity::Player,
            capability: id("origin-capability"),
            enabled: key("yes"),
        },
        RuleEffectKind::ActivateGrant {
            slot: grant,
            enabled: key("yes"),
        },
        RuleEffectKind::SupportApplicability {
            applicable: key("yes"),
        },
        RuleEffectKind::ProjectActorStat {
            actor: DeclaredSlot {
                declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
                slot: id("actor.supplied"),
            },
            stat: id("stat.effective-attribute"),
            value: key("level-value"),
        },
        RuleEffectKind::ProjectSkillParameter {
            skill: DeclaredSlot {
                declaration: SlotOwnerDefId::Gem(id("gem.support")),
                slot: id("undeclared-skill"),
            },
            parameter: DeclaredSlot {
                declaration: SlotOwnerDefId::Skill(id("skill.target")),
                slot: id("undeclared-input"),
            },
            value: key("level-value"),
        },
        RuleEffectKind::ProjectModifierTransform {
            stat: id("origin-value"),
            targets: vec![],
            order: BoundedInteger::new(0).unwrap(),
            operation: ModifierTransformOperation::Add,
            value: key("level-value"),
        },
    ] {
        let mut changed = rules.clone();
        program(&mut changed).effects[0].effect = effect;
        fails(
            &schema,
            &changed,
            "SupportOrigin context can only write its own",
        );
    }
}

#[test]
fn typed_scopes_and_version_gates_apply_to_false_guarded_effects_too() {
    let (schema, mut rules) = base();
    program(&mut rules).effects[0].effect = RuleEffectKind::Derive {
        entity: RuleEntity::Current,
        stat: id("skill-value"),
        value: key("level-value"),
    };
    fails(&schema, &rules, "stat target/context mismatch");
    let p = program(&mut rules);
    p.context = RuleEntityKind::Action;
    p.nodes.push(RuleNode {
        id: key("no"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    });
    p.effects[0].when = Some(key("no"));
    p.effects[0].effect = RuleEffectKind::Derive {
        entity: RuleEntity::SupportOrigin,
        stat: id("origin-value"),
        value: key("level-value"),
    };
    compile(&schema, &rules).unwrap();
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V11);
    fails(
        &schema,
        &rules,
        "preparation scopes require owned-domain-operations-v12",
    );
}
