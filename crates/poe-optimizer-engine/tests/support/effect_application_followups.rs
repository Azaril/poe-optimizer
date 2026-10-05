//! Source-input and staged-execution regression cases for the shared applications.
use super::fixture as support_fixture;
use super::*;
use poe_optimizer_core::{owned_definitions::*, owned_stages::*};
use poe_optimizer_engine::{owned_rules::RuleLimits, owned_supports::SupportPreparationLimits};

fn source_skill(
    f: &mut Fixture,
) -> (
    DeclaredSlot<ParameterSlotDefId>,
    DeclaredSlot<ChoiceSlotDefId>,
) {
    let supply = DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("application-skill"),
    };
    let grant = DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("application-grant"),
    };
    let parameter = parameter(SlotOwnerDefId::Skill(def("application-source")), "strength");
    let choice = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("application-source")),
        slot: def("scale"),
    };
    let ports = DeclaredSlots {
        parameters: DeclaredSet::complete(vec![parameter.clone()]),
        choices: DeclaredSet::complete(vec![choice.clone()]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    };
    f.schema
        .definitions
        .push(DefinitionDescriptor::Skill(known_entry(
            def("application-source"),
            SkillSchema {
                directly_selectable: false,
                declarations: ports,
            },
        )));
    let range = IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    };
    f.schema.slots.extend([
        SlotDescriptor::Parameter(known_entry(
            parameter.clone(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(range.clone()),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::Choice(known_entry(
            choice.clone(),
            ChoiceSlotSchema {
                value: ValueSchema::Integer(range),
                presence: SlotPresence::RequiredOnce,
                owners: vec![ChoiceOwnerScope::Skill],
            },
        )),
        SlotDescriptor::SkillGrant(known_entry(
            supply.clone(),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("application-source"),
                outputs: DeclaredSet::complete(vec![]),
            },
        )),
        SlotDescriptor::Grant(known_entry(
            grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(supply.clone()),
            },
        )),
    ]);
    for row in &mut f.schema.definitions {
        if let DefinitionDescriptor::Gem(row) = row
            && row.id == def("summoner")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.skills.members.push(def("application-source"));
            schema
                .declarations
                .skill_grants
                .members
                .push(supply.clone());
            schema.declarations.grants.members.push(grant.clone());
        }
    }
    f.owner_mut(&summoner_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("supply-application-skill"),
            context: RuleEntityKind::Actor,
            reads: vec![read("level", RuleReadSource::GemLevel)],
            nodes: vec![
                read_node("level", "level"),
                node(
                    "enabled",
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    },
                ),
            ],
            effects: vec![
                effect(
                    "activate",
                    RuleEffectKind::ActivateGrant {
                        slot: grant.clone(),
                        enabled: key("enabled"),
                    },
                ),
                effect(
                    "project",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: supply.clone(),
                        parameter: parameter.clone(),
                        value: key("level"),
                    },
                ),
            ],
        });
    f.owners.extend([
        DefinitionRules {
            owner: subject(def::<SkillDefinition>("application-source")),
            programs: DeclaredSet::complete(vec![]),
        },
        DefinitionRules {
            owner: SchemaSubject::Slot(SlotAddress::Grant(grant)),
            programs: DeclaredSet::complete(vec![]),
        },
    ]);
    for (use_id, scale) in [(30, 2), (31, 3)] {
        f.build.choices.push(MechanicChoice {
            owner: ChoiceOwner::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                provider: summoner_provider(use_id),
                slot: supply.clone(),
            }))),
            choice: ChoiceSelection {
                slot: choice.clone(),
                value: integer(scale),
            },
        });
    }
    (parameter, choice)
}

#[test]
fn generated_skill_source_uses_projected_inputs_and_exact_occurrence_choices() {
    let mut f = world();
    let (parameter, choice) = source_skill(&mut f);
    let mut row = application("generated");
    row.source = EffectApplicationSource::Skill {
        skill: def("application-source"),
    };
    row.program.reads[0].source = RuleReadSource::EffectSourceParameter {
        slot: parameter.clone(),
    };
    row.program.reads.push(read(
        "scale",
        RuleReadSource::EffectSourceChoice { slot: choice },
    ));
    row.program.nodes.extend([
        read_node("scale", "scale"),
        node(
            "scaled",
            RuleExpression::ScaleInteger {
                value: key("strength"),
                count: key("scale"),
            },
        ),
    ]);
    if let RuleEffectKind::Contribute { value, .. } = &mut row.program.effects[0].effect {
        *value = key("scaled");
    }
    let report = evaluate(&f, vec![row.clone()]);
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(number(&report, "delivered"), 60);
    let group = report
        .application_groups
        .iter()
        .find(|group| group.key.effect == key("damage"))
        .unwrap();
    assert_eq!(
        group.candidates.len(),
        2,
        "physical Gem roots are not duplicate Skill sources"
    );
    assert!(
        matches!(&group.co_winners[0].invocation.origin,RuleOrigin::EffectApplication { source:ConcreteEntity::Skill(target),.. }
        if matches!(target.as_ref(),SkillTarget::Generated(skill) if skill.provider==summoner_provider(31)))
    );
    let saved = f.build.choices.pop().unwrap();
    assert!(matches!(
        value(&evaluate(&f, vec![row.clone()]), "delivered"),
        EffectValue::Unresolved { .. }
    ));
    f.build.choices.push(saved);
    f.owner_mut(&summoner_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("supply-application-skill"))
        .unwrap()
        .effects
        .retain(|e| e.id != key("project"));
    row.program
        .nodes
        .iter_mut()
        .find(|node| node.id == key("active"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    // Existing required source-input gates remain authoritative. This does not
    // change the still-separate readiness contract merely to skip a buff.
    assert!(matches!(
        value(&evaluate(&f, vec![row.clone()]), "delivered"),
        EffectValue::Unresolved { .. }
    ));
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::Parameter(entry) = slot
            && entry.id == parameter
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.presence = SlotPresence::OptionalOnce;
        }
    }
    // An admitted source's optional, unused strength is genuinely lazy.
    assert_eq!(number(&evaluate(&f, vec![row]), "delivered"), 0);
}

#[test]
fn quantity_scaling_is_recipient_specific_and_keeps_negative_co_winners() {
    let mut f = world();
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Stat(row) = definition
            && row.id == def("delivered")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.value = ComputedValueType::Quantity { unit: def("count") };
        }
    }
    let quantity = |n| ParameterValue::Quantity(FiniteQuantity::new(n, def("count")).unwrap());
    let aggregate = f
        .owner_mut(&class_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("aggregate-delivered"))
        .unwrap();
    aggregate.reads[0].value_type = ComputedValueType::Quantity { unit: def("count") };
    if let RuleReadSource::Contributions { empty, .. } = &mut aggregate.reads[0].source {
        *empty = quantity(0.);
    }
    f.owner_mut(&class_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("player-scale"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![literal("two", 2)],
            effects: vec![derive("scale", RuleEntity::Current, "child-level", "two")],
        });
    let mut a = application("a");
    a.targets
        .push(EffectApplicationTarget::OwnedSlot { slot: child_slot() });
    a.program.reads.push(read(
        "recipient",
        RuleReadSource::Stat {
            entity: RuleEntity::Actor,
            stat: def("child-level"),
        },
    ));
    a.program.nodes.extend([
        node(
            "point",
            RuleExpression::Literal {
                value: quantity(-1.),
            },
        ),
        read_node("recipient", "recipient"),
        node(
            "source-quantity",
            RuleExpression::ScaleInteger {
                value: key("point"),
                count: key("strength"),
            },
        ),
        node(
            "scaled-quantity",
            RuleExpression::ScaleInteger {
                value: key("source-quantity"),
                count: key("recipient"),
            },
        ),
    ]);
    if let RuleEffectKind::Contribute { value, .. } = &mut a.program.effects[0].effect {
        *value = key("scaled-quantity");
    }
    let mut b = a.clone();
    b.id = key("b");
    let report = evaluate(&f, vec![a, b]);
    for group in report
        .application_groups
        .iter()
        .filter(|group| group.key.effect == key("damage"))
    {
        let RuleOrigin::EffectApplicationGroup {
            recipient: ConcreteEntity::Actor(actor),
            ..
        } = &group.key.invocation.origin
        else {
            panic!("exact actor group")
        };
        let expected = if *actor == ActorKey::Player {
            -22.
        } else if *actor == child_actor(30) {
            -121.
        } else {
            assert_eq!(*actor, child_actor(31));
            -220.
        };
        assert_eq!(
            group.value,
            EffectValue::Known {
                value: quantity(expected)
            }
        );
        assert_eq!(group.co_winners.len(), 2);
    }
    assert_eq!(
        value(&report, "delivered"),
        &EffectValue::Known {
            value: quantity(-22.)
        }
    );
}

#[test]
fn inline_boolean_edges_are_bounded_before_program_cloning() {
    let f = world();
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut row = application("bounded");
    row.program.nodes.push(node(
        "true",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(true),
        },
    ));
    row.program
        .nodes
        .iter_mut()
        .find(|node| node.id == key("active"))
        .unwrap()
        .expression = RuleExpression::All {
        values: vec![key("true"), key("true")],
    };
    let error = CompiledRulePackage::compile(
        &input(&f, &schema, DeclaredSet::complete(vec![row])),
        &schema,
        RuleLimits {
            max_edges: 1,
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("application.nodes") && error.contains("list edge limit"),
        "{error}"
    );
}

fn staged_plan(active: bool) -> OwnedSupportPreparationPlan<OwnedDefinitionSchemaPackage> {
    use poe_optimizer_data::{
        owned_rules::OwnedRulePackage, owned_stages::OwnedEvaluationStages,
        owned_support_inputs::OwnedSupportInputBindings, owned_supports::OwnedSupportPreparation,
    };
    let mut f = support_fixture::generated_fixture();
    f.owner_mut(&class_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("application-total"),
            context: RuleEntityKind::Actor,
            reads: vec![contributions(
                "incoming",
                RuleEntity::Current,
                "actor-total",
            )],
            nodes: vec![read_node("value", "incoming")],
            effects: vec![derive("total", RuleEntity::Current, "actor-total", "value")],
        });
    let origin = &mut f
        .owner_mut(&subject(def::<GemDefinition>("support")))
        .programs
        .members[0];
    origin.reads.push(read(
        "buff",
        RuleReadSource::Stat {
            entity: RuleEntity::Player,
            stat: def("actor-total"),
        },
    ));
    origin.nodes.extend([
        read_node("buff", "buff"),
        literal("threshold", 20),
        node(
            "enabled",
            RuleExpression::Compare {
                operation: RuleComparison::Greater,
                left: key("buff"),
                right: key("threshold"),
            },
        ),
        node(
            "inverse",
            RuleExpression::Subtract {
                left: key("base"),
                right: key("level"),
            },
        ),
    ]);
    origin
        .nodes
        .iter_mut()
        .find(|node| node.id == key("effective"))
        .unwrap()
        .expression = RuleExpression::Select {
        condition: key("enabled"),
        when_true: key("level"),
        when_false: key("inverse"),
    };
    let mut args = support_fixture::compile_inputs(&f, support_fixture::target(30, "first"));
    let mut application = application("support-buff");
    application.source = EffectApplicationSource::Skill {
        skill: def("ability"),
    };
    application.program.reads[0].source = RuleReadSource::EffectSourceParameter {
        slot: parameter(SlotOwnerDefId::Skill(def("ability")), "level"),
    };
    application
        .program
        .nodes
        .iter_mut()
        .find(|node| node.id == key("active"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(active),
    };
    application.program.effects.truncate(1);
    application.stacking.truncate(1);
    if let RuleEffectKind::Contribute { stat, .. } = &mut application.program.effects[0].effect {
        *stat = def("actor-total");
    }
    let mut rules = args.rules.input().clone();
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V15);
    rules.effect_applications = Some(DeclaredSet::complete(vec![application]));
    let stored =
        OwnedRulePackage::new(rules, args.definitions.as_ref(), Default::default()).unwrap();
    args.rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, args.definitions.as_ref(), Default::default())
            .unwrap(),
    );
    let mut stage = args.stages.input().clone();
    stage.rules = *stored.identity();
    stage.effect_applications = Some(DeclaredSet::complete(vec![StagedEffectApplication {
        application: key("support-buff"),
        stage: key("prepare"),
    }]));
    args.stages = Arc::new(
        OwnedEvaluationStages::new(
            stage,
            args.definitions.as_ref(),
            &stored,
            &args.routing,
            Default::default(),
        )
        .unwrap(),
    );
    let mut preparation = args.preparation.input().clone();
    preparation.rules = *stored.identity();
    args.preparation = Arc::new(
        OwnedSupportPreparation::new(
            preparation,
            args.definitions.as_ref(),
            &stored,
            Default::default(),
        )
        .unwrap(),
    );
    let mut inputs = args.inputs.input().clone();
    inputs.rules = *stored.identity();
    inputs.stages = *args.stages.identity();
    inputs.preparation = *args.preparation.identity();
    args.inputs = Arc::new(
        OwnedSupportInputBindings::new(
            inputs,
            args.definitions.as_ref(),
            &stored,
            &args.preparation,
            &args.stages,
            Default::default(),
        )
        .unwrap(),
    );
    OwnedSupportPreparationPlan::compile(
        args,
        Default::default(),
        SupportPreparationLimits::default(),
    )
    .unwrap()
}

#[test]
fn classified_applications_run_inside_the_existing_support_preparation_graph() {
    let active = staged_plan(true);
    let inactive = staged_plan(false);
    let mut scratch = active.new_scratch();
    let report = active.evaluate(&mut scratch).unwrap();
    assert!(report.gaps.is_empty());
    assert_eq!(
        support_fixture::prepared(&report).selected[0].assignment,
        occurrence(61)
    );
    let other = inactive.evaluate(&mut scratch).unwrap();
    assert_eq!(
        support_fixture::prepared(&other).selected[0].assignment,
        occurrence(60)
    );
    assert_eq!(active.evaluate(&mut scratch).unwrap(), report);
}
