//! Actor templates supply distinct abilities through the ordinary owned plan.
//! Every input is synthetic; no import token, source engine, or family dispatch is used.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::{owned_plan::*, owned_rules::*};
use std::sync::Arc;

fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn actor_owner() -> SchemaSubject {
    subject(def::<ActorDefinition>("family"))
}
fn slot<K: DefinitionDomain>(owner: SlotOwnerDefId, name: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: def(name),
    }
}
fn supply(name: &str) -> DeclaredSlot<SkillGrantSlotDefId> {
    slot(SlotOwnerDefId::Actor(def("family")), name)
}
fn activation(name: &str) -> DeclaredSlot<GrantSlotDefId> {
    slot(SlotOwnerDefId::Actor(def("family")), name)
}
fn ability_input(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Skill(def("ability")), name)
}
fn ability_output() -> DeclaredSlot<ActionOutputDefId> {
    slot(SlotOwnerDefId::Skill(def("ability")), "ability-output")
}
fn entered_actor(use_id: u64) -> ProviderKey {
    let mut key = summoner_provider(use_id);
    key.grant_path.push(child_grant());
    key
}
fn selected(use_id: u64, name: &str) -> ActionSelection {
    let mut provider = entered_actor(use_id);
    provider.grant_path.push(activation(name));
    ActionSelection {
        action: ActionKey {
            actor: child_actor(use_id),
            provider,
            output: ability_output(),
        },
        part: def("part"),
        mode: def("mode"),
        stat_set: def("set"),
    }
}
fn selected_value(use_id: u64, name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Action(Box::new(selected(use_id, name))),
        stat: def("ability-result"),
    }
}
fn definition_ports(f: &mut Fixture) -> &mut DeclaredSlots {
    f.schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Actor(e) if e.id == def("family") => match &mut e.schema {
                SchemaState::Known(s) => Some(&mut s.declarations),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}
fn population(f: &mut Fixture) -> &mut ActorSlotSchema {
    f.schema
        .slots
        .iter_mut()
        .find_map(|d| match d {
            SlotDescriptor::Actor(e) if e.id == child_slot() => match &mut e.schema {
                SchemaState::Known(s) => Some(s),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}
fn activation_program<'a>(f: &'a mut Fixture, name: &str) -> &'a mut RuleProgram {
    f.owner_mut(&actor_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(name))
        .unwrap()
}
fn rules(f: &mut Fixture, owner: SchemaSubject, programs: Vec<RuleProgram>) {
    f.owners.push(DefinitionRules {
        owner,
        programs: DeclaredSet::complete(programs),
    });
}
fn boolean(id: &str, value: bool) -> RuleNode {
    node(
        id,
        RuleExpression::Literal {
            value: ParameterValue::Boolean(value),
        },
    )
}
fn add_supply(f: &mut Fixture, name: &str) {
    definition_ports(f).skill_grants.members.push(supply(name));
    definition_ports(f).grants.members.push(activation(name));
    f.schema.slots.extend([
        SlotDescriptor::SkillGrant(entry(
            supply(name),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("ability"),
                outputs: DeclaredSet::complete(vec![ability_output()]),
            },
        )),
        SlotDescriptor::Grant(entry(
            activation(name),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(supply(name)),
            },
        )),
    ]);
    f.owner_mut(&actor_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key(name),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "actor-level",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("child-plus"),
                },
            )],
            nodes: vec![
                read_node("level", "actor-level"),
                literal("zero", 0),
                boolean("enabled", true),
            ],
            effects: vec![
                effect(
                    "activate",
                    RuleEffectKind::ActivateGrant {
                        slot: activation(name),
                        enabled: key("enabled"),
                    },
                ),
                effect(
                    "level",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: supply(name),
                        parameter: ability_input("level"),
                        value: key("level"),
                    },
                ),
                effect(
                    "quality",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: supply(name),
                        parameter: ability_input("quality"),
                        value: key("zero"),
                    },
                ),
            ],
        });
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.build.items.clear();
    f.build.equipment.clear();
    f.owner_mut(&class_owner()).programs.members.clear();
    for definition in &mut f.schema.definitions {
        if let DefinitionDescriptor::Metric(entry) = definition
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.actor_roles.push(MetricActorRole::Owned);
        }
    }
    f.add_generated_actors();
    population(&mut f).provider_definition = Some(def("family"));
    population(&mut f).skills = DeclaredSet::complete(vec![def("ability")]);
    let mut skill_ports = ports();
    skill_ports.parameters =
        DeclaredSet::complete(vec![ability_input("level"), ability_input("quality")]);
    skill_ports.outputs = DeclaredSet::complete(vec![ability_output()]);
    f.schema.definitions.extend([
        DefinitionDescriptor::Actor(entry(
            def("family"),
            ActorSchema {
                declarations: ports(),
            },
        )),
        DefinitionDescriptor::Skill(entry(
            def("ability"),
            SkillSchema {
                directly_selectable: false,
                declarations: skill_ports,
            },
        )),
        DefinitionDescriptor::Stat(entry(
            def("ability-result"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Action],
            },
        )),
    ]);
    for name in ["level", "quality"] {
        f.schema.slots.push(SlotDescriptor::Parameter(entry(
            ability_input(name),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )));
    }
    f.schema.slots.push(SlotDescriptor::ActionOutput(entry(
        ability_output(),
        ActionOutputSchema {
            actor_role: DeclaredActorRole::ProviderActor,
            parts: DeclaredSet::complete(vec![def("part")]),
            modes: DeclaredSet::complete(vec![def("mode")]),
            stat_sets: DeclaredSet::complete(vec![def("set")]),
            choices: empty(),
        },
    )));
    rules(&mut f, actor_owner(), vec![]);
    rules(
        &mut f,
        subject(def::<SkillDefinition>("ability")),
        vec![RuleProgram {
            id: key("calculate"),
            context: RuleEntityKind::Action,
            reads: vec![read(
                "level",
                RuleReadSource::Parameter {
                    slot: ability_input("level"),
                },
            )],
            nodes: vec![read_node("level", "level")],
            effects: vec![derive(
                "result",
                RuleEntity::Current,
                "ability-result",
                "level",
            )],
        }],
    );
    rules(
        &mut f,
        SchemaSubject::Slot(SlotAddress::ActionOutput(ability_output())),
        vec![],
    );
    f.routes.push(ActionOutputRoutes {
        output: ability_output(),
        source_selectors: Some(empty()),
        routes: empty(),
    });
    for name in ["first", "second"] {
        add_supply(&mut f, name);
        for use_id in [30, 31] {
            f.queries.requests.push(MetricRequest {
                id: QueryId::new(format!("{use_id}-{name}")).unwrap(),
                metric: def("requested"),
                target: MetricTarget::Action(Box::new(selected(use_id, name))),
            });
        }
    }
    f
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn value<'a>(report: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    let mut found = report.values.iter().filter(|v| &v.key == key);
    let result = &found
        .next()
        .unwrap_or_else(|| panic!("missing {key:?}: {report:?}"))
        .value;
    assert!(found.next().is_none());
    result
}

#[test]
fn shared_actor_definitions_and_repeated_ability_slots_keep_exact_inputs_and_parallel_results() {
    let f = fixture();
    let plan = Arc::new(f.compile().unwrap());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for (use_id, expected) in [(30, 12), (31, 21)] {
        for name in ["first", "second"] {
            assert_eq!(
                value(&report, &selected_value(use_id, name)),
                &EffectValue::Known {
                    value: integer(expected)
                }
            );
            assert_eq!(
                value(
                    &report,
                    &PlanValueKey::SkillParameter {
                        skill: Box::new(GeneratedSkillKey {
                            provider: entered_actor(use_id),
                            slot: supply(name)
                        }),
                        parameter: ability_input("quality"),
                    }
                ),
                &EffectValue::Known { value: integer(0) }
            );
        }
    }
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let plan = Arc::clone(&plan);
            std::thread::spawn(move || plan.evaluate(&mut plan.new_scratch()).unwrap())
        })
        .collect();
    for worker in workers {
        assert_eq!(report, worker.join().unwrap());
    }
    let mut reordered = f;
    reordered.schema.definitions.reverse();
    reordered.schema.slots.reverse();
    reordered.owners.reverse();
    assert_eq!(report, evaluate(&reordered));
}

#[test]
fn inactive_actor_and_ability_gates_are_distinct_from_missing_activation_and_required_inputs() {
    for mode in [
        "actor-inactive",
        "ability-inactive",
        "activation-missing",
        "input-missing",
    ] {
        let mut f = fixture();
        match mode {
            "actor-inactive" => {
                f.build.gems[0].parameters[0].value = ParameterValue::Boolean(false)
            }
            "ability-inactive" => {
                activation_program(&mut f, "first").nodes[2] = boolean("enabled", false)
            }
            "activation-missing" => activation_program(&mut f, "first")
                .effects
                .retain(|e| e.id != key("activate")),
            "input-missing" => activation_program(&mut f, "first")
                .effects
                .retain(|e| e.id != key("quality")),
            _ => unreachable!(),
        }
        let report = evaluate(&f);
        let affected = value(&report, &selected_value(30, "first"));
        if mode.ends_with("inactive") {
            assert_eq!(affected, &EffectValue::Inactive, "{mode}");
        } else {
            assert!(
                matches!(affected, EffectValue::Unresolved { .. }),
                "{mode}: {affected:?}"
            );
        }
        if mode == "actor-inactive" {
            assert_eq!(
                value(&report, &selected_value(31, "first")),
                &EffectValue::Known { value: integer(21) }
            );
        }
    }
}

#[test]
fn ability_input_projection_type_mismatch_is_invalid_before_evaluation() {
    let mut f = fixture();
    activation_program(&mut f, "first").effects[1] = effect(
        "level",
        RuleEffectKind::ProjectSkillParameter {
            skill: supply("first"),
            parameter: ability_input("level"),
            value: key("enabled"),
        },
    );
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let rules = RulePackageInput {
        existing_actor_rules: None,
        ordered_contributions: None,
        effect_applications: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("test"),
        semantics_version: key("test"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: f.owners,
        tables: vec![],
        receivers: empty(),
    };
    assert!(CompiledRulePackage::compile(&rules, &schema, RuleLimits::default()).is_err());
}

#[test]
fn alternate_grants_cannot_duplicate_actor_or_ability_occurrences() {
    for actor in [false, true] {
        let mut f = fixture();
        let grant = if actor {
            slot(SlotOwnerDefId::Gem(def("summoner")), "alternate-actor")
        } else {
            activation("alternate-ability")
        };
        if actor {
            let DefinitionDescriptor::Gem(entry) = f
                .schema
                .definitions
                .iter_mut()
                .find(|d| d.address() == def::<GemDefinition>("summoner").address())
                .unwrap()
            else {
                panic!()
            };
            let SchemaState::Known(schema) = &mut entry.schema else {
                panic!()
            };
            schema.declarations.grants.members.push(grant.clone());
        } else {
            definition_ports(&mut f).grants.members.push(grant.clone());
        }
        f.schema.slots.push(SlotDescriptor::Grant(entry(
            grant,
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: if actor {
                    GrantTarget::Actor(child_slot())
                } else {
                    GrantTarget::Skill(supply("first"))
                },
            },
        )));
        let error = f
            .compile()
            .err()
            .expect("ambiguous supply rejected")
            .to_string();
        assert!(
            error.contains(if actor {
                "ambiguous actor supply"
            } else {
                "ambiguous skill supply"
            }),
            "{error}"
        );
    }
}

#[test]
fn membership_is_positive_and_legacy_population_does_not_acquire_actor_definition() {
    let mut f = fixture();
    population(&mut f).skills.members.clear();
    assert!(OwnedDefinitionSchemaPackage::new(f.schema, OwnedSchemaLimits::default()).is_err());
    let mut legacy = fixture();
    population(&mut legacy).provider_definition = None;
    legacy.queries.requests.clear();
    let report = evaluate(&legacy);
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::UnresolvedActivation)
    );
    assert!(
        !report
            .effects
            .iter()
            .any(|e| e.key.invocation.owner == actor_owner())
    );
    assert!(report.values.iter().any(|v| v.key
        == PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(child_actor(30)),
            stat: def("child-plus"),
        }));
}

#[test]
fn topology_expansion_is_bounded_before_child_allocation() {
    let mut f = fixture();
    f.queries.requests.clear();
    let limits = PlanLimits {
        max_providers: 3,
        ..PlanLimits::default()
    };
    assert!(matches!(
        f.compile_with(limits),
        Err(PlanError::Limit("providers"))
    ));
    let mut limits = PlanLimits::default();
    limits.binding.input.max_provider_steps = 1;
    // Query binding independently enforces the same depth. Clear queries to
    // exercise discovery's pre-expansion check rather than selected input validation.
    assert!(matches!(
        f.compile_with(limits),
        Err(PlanError::Limit("provider depth"))
    ));
}

fn nested_slot() -> DeclaredSlot<ActorSlotDefId> {
    slot(SlotOwnerDefId::Actor(def("family")), "nested-population")
}
fn add_nested_family(f: &mut Fixture) {
    let nested_grant = activation("nested-actor");
    let nested_supply = slot(SlotOwnerDefId::Actor(def("other-family")), "nested-ability");
    let nested_activation = slot(
        SlotOwnerDefId::Actor(def("other-family")),
        "nested-activate",
    );
    let nested_parameter = parameter(SlotOwnerDefId::Skill(def("other-ability")), "nested-input");
    definition_ports(f).actors.members.push(nested_slot());
    definition_ports(f)
        .grants
        .members
        .push(nested_grant.clone());
    let mut nested_ports = ports();
    nested_ports.grants.members.push(nested_activation.clone());
    nested_ports
        .skill_grants
        .members
        .push(nested_supply.clone());
    let mut ability_ports = ports();
    ability_ports
        .parameters
        .members
        .push(nested_parameter.clone());
    f.schema.definitions.extend([
        DefinitionDescriptor::Actor(entry(
            def("other-family"),
            ActorSchema {
                declarations: nested_ports,
            },
        )),
        DefinitionDescriptor::Skill(entry(
            def("other-ability"),
            SkillSchema {
                directly_selectable: false,
                declarations: ability_ports,
            },
        )),
        DefinitionDescriptor::Stat(entry(
            def("nested-result"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor],
            },
        )),
    ]);
    f.schema.slots.extend([
        SlotDescriptor::Actor(entry(
            nested_slot(),
            ActorSlotSchema {
                provider_definition: Some(def("other-family")),
                skills: DeclaredSet::complete(vec![def("other-ability")]),
                outputs: empty(),
            },
        )),
        SlotDescriptor::Grant(entry(
            nested_grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Actor(nested_slot()),
            },
        )),
        SlotDescriptor::Grant(entry(
            nested_activation.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(nested_supply.clone()),
            },
        )),
        SlotDescriptor::SkillGrant(entry(
            nested_supply.clone(),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("other-ability"),
                outputs: empty(),
            },
        )),
        SlotDescriptor::Parameter(entry(
            nested_parameter.clone(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
    ]);
    f.owner_mut(&actor_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("nested"),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "actor-value",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("child-plus"),
                },
            )],
            nodes: vec![
                read_node("actor-value", "actor-value"),
                boolean("enabled", true),
            ],
            effects: vec![
                effect(
                    "project",
                    RuleEffectKind::ProjectActorStat {
                        actor: nested_slot(),
                        stat: def("child-level"),
                        value: key("actor-value"),
                    },
                ),
                effect(
                    "activate",
                    RuleEffectKind::ActivateGrant {
                        slot: nested_grant,
                        enabled: key("enabled"),
                    },
                ),
            ],
        });
    // Retain actor-slot baseline programs on the nested population too.
    let baseline = f
        .owner_mut(&SchemaSubject::Slot(SlotAddress::Actor(child_slot())))
        .programs
        .members
        .clone();
    rules(
        f,
        SchemaSubject::Slot(SlotAddress::Actor(nested_slot())),
        baseline,
    );
    rules(
        f,
        subject(def::<ActorDefinition>("other-family")),
        vec![RuleProgram {
            id: key("other-supply"),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "current",
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def("child-plus"),
                },
            )],
            nodes: vec![read_node("current", "current"), boolean("enabled", true)],
            effects: vec![
                effect(
                    "project",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: nested_supply,
                        parameter: nested_parameter.clone(),
                        value: key("current"),
                    },
                ),
                effect(
                    "activate",
                    RuleEffectKind::ActivateGrant {
                        slot: nested_activation,
                        enabled: key("enabled"),
                    },
                ),
            ],
        }],
    );
    rules(
        f,
        subject(def::<SkillDefinition>("other-ability")),
        vec![RuleProgram {
            id: key("nested-consumer"),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "input",
                RuleReadSource::Parameter {
                    slot: nested_parameter,
                },
            )],
            nodes: vec![read_node("input", "input")],
            effects: vec![derive(
                "result",
                RuleEntity::Current,
                "nested-result",
                "input",
            )],
        }],
    );
}

#[test]
fn nested_actor_definitions_keep_immediate_parent_and_current_actor_inputs() {
    let mut f = fixture();
    add_nested_family(&mut f);
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for (use_id, expected) in [(30, 13), (31, 22)] {
        let actor = ActorKey::Owned(Box::new(OwnedActorKey {
            provider: entered_actor(use_id),
            slot: nested_slot(),
        }));
        assert_eq!(
            value(
                &report,
                &PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor),
                    stat: def("nested-result")
                }
            ),
            &EffectValue::Known {
                value: integer(expected)
            }
        );
        assert_eq!(
            value(&report, &selected_value(use_id, "first")),
            &EffectValue::Known {
                value: integer(expected - 1)
            }
        );
    }
}

#[test]
fn potential_recursion_is_rejected_even_when_its_runtime_grant_is_false() {
    let mut f = fixture();
    add_nested_family(&mut f);
    for slot in &mut f.schema.slots {
        if let SlotDescriptor::Actor(e) = slot
            && e.id == nested_slot()
        {
            let SchemaState::Known(schema) = &mut e.schema else {
                panic!()
            };
            schema.provider_definition = Some(def("family"));
            schema.skills = DeclaredSet::complete(vec![def("ability")]);
        }
    }
    f.owner_mut(&actor_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("nested"))
        .unwrap()
        .nodes[1] = boolean("enabled", false);
    let error = f
        .compile()
        .err()
        .expect("potential recursion rejected")
        .to_string();
    assert!(
        error.contains("recursive potential supply cycle"),
        "{error}"
    );
}

#[test]
fn wrong_actor_action_pair_cannot_consume_a_siblings_ability() {
    let mut f = fixture();
    let MetricTarget::Action(action) = &mut f
        .queries
        .requests
        .iter_mut()
        .find(|q| q.id == QueryId::new("30-first").unwrap())
        .unwrap()
        .target
    else {
        panic!()
    };
    action.action.actor = child_actor(31);
    assert!(f.compile().is_err());
}

#[test]
fn summoner_actor_and_generated_skill_choices_have_no_scope_fallback() {
    let mut f = fixture();
    let parent_choice = slot(SlotOwnerDefId::Gem(def("summoner")), "choice");
    let actor_choice = slot(SlotOwnerDefId::Actor(def("family")), "choice");
    let ability_choice = slot(SlotOwnerDefId::Skill(def("ability")), "choice");
    definition_ports(&mut f)
        .choices
        .members
        .push(actor_choice.clone());
    for d in &mut f.schema.definitions {
        match d {
            DefinitionDescriptor::Gem(e) if e.id == def("summoner") => {
                let SchemaState::Known(schema) = &mut e.schema else {
                    panic!()
                };
                schema
                    .declarations
                    .choices
                    .members
                    .push(parent_choice.clone());
            }
            DefinitionDescriptor::Skill(e) if e.id == def("ability") => {
                let SchemaState::Known(schema) = &mut e.schema else {
                    panic!()
                };
                schema
                    .declarations
                    .choices
                    .members
                    .push(ability_choice.clone());
            }
            _ => {}
        }
    }
    for (choice, scope) in [
        (
            parent_choice.clone(),
            ChoiceOwnerScope::Provider(ProviderRole::SkillUse),
        ),
        (
            actor_choice.clone(),
            ChoiceOwnerScope::Provider(ProviderRole::SkillUse),
        ),
        (ability_choice.clone(), ChoiceOwnerScope::Skill),
    ] {
        f.schema.slots.push(SlotDescriptor::Choice(entry(
            choice,
            ChoiceSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::OptionalOnce,
                owners: vec![scope],
            },
        )));
    }
    for name in ["first", "second"] {
        activation_program(&mut f, name).reads[0].source = RuleReadSource::Choice {
            slot: actor_choice.clone(),
        };
    }
    let program = &mut f
        .owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members[0];
    program.reads.push(read(
        "ability-choice",
        RuleReadSource::Choice {
            slot: ability_choice.clone(),
        },
    ));
    program
        .nodes
        .push(read_node("ability-choice", "ability-choice"));
    program.nodes.push(node(
        "sum",
        RuleExpression::Add {
            left: key("level"),
            right: key("ability-choice"),
        },
    ));
    program.effects[0] = derive("result", RuleEntity::Current, "ability-result", "sum");
    for (use_id, actor_value, first, second) in [(30, 7, 71, 72), (31, 9, 91, 92)] {
        f.build.choices.extend([
            MechanicChoice {
                owner: ChoiceOwner::Provider(summoner_provider(use_id)),
                choice: ChoiceSelection {
                    slot: parent_choice.clone(),
                    value: integer(99),
                },
            },
            MechanicChoice {
                owner: ChoiceOwner::Provider(entered_actor(use_id)),
                choice: ChoiceSelection {
                    slot: actor_choice.clone(),
                    value: integer(actor_value),
                },
            },
        ]);
        for (name, amount) in [("first", first), ("second", second)] {
            f.build.choices.push(MechanicChoice {
                owner: ChoiceOwner::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider: entered_actor(use_id),
                    slot: supply(name),
                }))),
                choice: ChoiceSelection {
                    slot: ability_choice.clone(),
                    value: integer(amount),
                },
            });
        }
    }
    let report = evaluate(&f);
    for (use_id, name, expected) in [
        (30, "first", 78),
        (30, "second", 79),
        (31, "first", 100),
        (31, "second", 101),
    ] {
        assert_eq!(
            value(&report, &selected_value(use_id, name)),
            &EffectValue::Known {
                value: integer(expected)
            }
        );
    }
    f.build.choices.retain(|c| {
        !(c.owner == ChoiceOwner::Provider(entered_actor(30)) && c.choice.slot == actor_choice)
    });
    let missing = evaluate(&f);
    assert!(matches!(
        value(&missing, &selected_value(30, "first")),
        EffectValue::Unresolved { .. }
    ));
    assert_eq!(
        value(&missing, &selected_value(31, "first")),
        &EffectValue::Known {
            value: integer(100)
        }
    );
}

#[test]
fn actor_potential_members_include_abilities_supplied_by_another_current_actor_ability() {
    let mut f = fixture();
    let child_supply = slot(SlotOwnerDefId::Skill(def("ability")), "descendant-supply");
    let child_grant = slot(SlotOwnerDefId::Skill(def("ability")), "descendant-grant");
    let child_input = parameter(SlotOwnerDefId::Skill(def("descendant")), "level");
    population(&mut f).skills.members.push(def("descendant"));
    for d in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(e) = d
            && e.id == def("ability")
        {
            let SchemaState::Known(schema) = &mut e.schema else {
                panic!()
            };
            schema
                .declarations
                .skill_grants
                .members
                .push(child_supply.clone());
            schema.declarations.grants.members.push(child_grant.clone());
        }
    }
    let mut child_ports = ports();
    child_ports.parameters.members.push(child_input.clone());
    f.schema.definitions.push(DefinitionDescriptor::Skill(entry(
        def("descendant"),
        SkillSchema {
            directly_selectable: false,
            declarations: child_ports,
        },
    )));
    f.schema.slots.extend([
        SlotDescriptor::SkillGrant(entry(
            child_supply.clone(),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("descendant"),
                outputs: empty(),
            },
        )),
        SlotDescriptor::Grant(entry(
            child_grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(child_supply.clone()),
            },
        )),
        SlotDescriptor::Parameter(entry(
            child_input.clone(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
    ]);
    f.owner_mut(&subject(def::<SkillDefinition>("ability")))
        .programs
        .members
        .push(RuleProgram {
            id: key("descendant"),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "level",
                RuleReadSource::Parameter {
                    slot: ability_input("level"),
                },
            )],
            nodes: vec![read_node("level", "level"), boolean("enabled", true)],
            effects: vec![
                effect(
                    "level",
                    RuleEffectKind::ProjectSkillParameter {
                        skill: child_supply.clone(),
                        parameter: child_input.clone(),
                        value: key("level"),
                    },
                ),
                effect(
                    "activate",
                    RuleEffectKind::ActivateGrant {
                        slot: child_grant,
                        enabled: key("enabled"),
                    },
                ),
            ],
        });
    rules(
        &mut f,
        subject(def::<SkillDefinition>("descendant")),
        vec![],
    );
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for (use_id, expected) in [(30, 12), (31, 21)] {
        for name in ["first", "second"] {
            assert_eq!(
                value(
                    &report,
                    &PlanValueKey::SkillParameter {
                        skill: Box::new(GeneratedSkillKey {
                            provider: selected(use_id, name).action.provider,
                            slot: child_supply.clone()
                        }),
                        parameter: child_input.clone(),
                    }
                ),
                &EffectValue::Known {
                    value: integer(expected)
                }
            );
        }
    }
    // Descendant supply cannot be silently removed just because the enclosing
    // Actor's complete potential set omits it. No query selects the descendant.
    population(&mut f)
        .skills
        .members
        .retain(|id| id != &def("descendant"));
    f.queries.requests.clear();
    let error = f
        .compile()
        .err()
        .expect("undeclared descendant rejected")
        .to_string();
    assert!(
        error.contains("declared potential supply is unavailable"),
        "{error}"
    );
}

#[test]
fn old_operation_plan_identity_and_legacy_actor_execution_remain_reproducible() {
    let mut f = Fixture::new();
    f.schema.schema_version = poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_V2;
    f.add_generated_actors();
    let new = f
        .compile_with_operations(PlanLimits::default(), OWNED_RULE_OPERATIONS_V11)
        .unwrap();
    assert_eq!(
        new.identity(),
        poe_optimizer_core::owned_content::digest_owned(
            "owned-effect-plan-v8",
            new.bindings(),
            PlanLimits::default().max_wire_bytes,
        )
        .unwrap()
    );
    let new_report = new.evaluate(&mut new.new_scratch()).unwrap();
    for (version, domain) in [
        (OWNED_RULE_OPERATIONS_V6, "owned-effect-plan-v6"),
        (OWNED_RULE_OPERATIONS_V7, "owned-effect-plan-v6"),
        (OWNED_RULE_OPERATIONS_V8, "owned-effect-plan-v6"),
        (OWNED_RULE_OPERATIONS_V9, "owned-effect-plan-v6"),
        (OWNED_RULE_OPERATIONS_V10, "owned-effect-plan-v7"),
    ] {
        let old = f
            .compile_with_operations(PlanLimits::default(), version)
            .unwrap();
        assert_eq!(
            old.identity(),
            poe_optimizer_core::owned_content::digest_owned(
                domain,
                old.bindings(),
                PlanLimits::default().max_wire_bytes
            )
            .unwrap()
        );
        let old_report = old.evaluate(&mut old.new_scratch()).unwrap();
        assert_eq!(old_report.values, new_report.values);
        assert_eq!(old_report.effects, new_report.effects);
        assert_eq!(old_report.gaps, new_report.gaps);
    }
}

#[test]
fn actor_rules_cannot_be_labeled_as_legacy_operations() {
    let f = fixture();
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let mut rules = RulePackageInput {
        existing_actor_rules: None,
        ordered_contributions: None,
        effect_applications: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("test"),
        semantics_version: key("test"),
        operations_version: key(OWNED_RULE_OPERATIONS_V10),
        definitions: schema.identity().clone(),
        owners: f.owners,
        tables: vec![],
        receivers: empty(),
    };
    let error = CompiledRulePackage::compile(&rules, &schema, RuleLimits::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("actor-owned supply requires owned-domain-operations-v11"),
        "{error}"
    );
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V11);
    CompiledRulePackage::compile(&rules, &schema, RuleLimits::default()).unwrap();
    rules.operations_version = key("owned-domain-operations-v999");
    assert!(
        CompiledRulePackage::compile(&rules, &schema, RuleLimits::default())
            .unwrap_err()
            .to_string()
            .contains("unsupported operation version")
    );
}

#[test]
fn equal_dimension_does_not_allow_wrong_unit_ability_projection() {
    let mut f = fixture();
    for name in ["unit-a", "unit-b"] {
        f.schema.definitions.push(DefinitionDescriptor::Unit(entry(
            def(name),
            UnitSchema {
                dimension: UnitDimension::Count,
            },
        )));
    }
    for d in &mut f.schema.slots {
        if let SlotDescriptor::Parameter(e) = d
            && e.id == ability_input("quality")
        {
            let SchemaState::Known(schema) = &mut e.schema else {
                panic!()
            };
            schema.value = ValueSchema::Quantity(QuantityRange {
                minimum: FiniteQuantity::new(0.0, def("unit-a")).unwrap(),
                maximum: FiniteQuantity::new(100.0, def("unit-a")).unwrap(),
            });
        }
    }
    for name in ["first", "second"] {
        activation_program(&mut f, name).nodes[1] = node(
            "zero",
            RuleExpression::Literal {
                value: ParameterValue::Quantity(FiniteQuantity::new(0.0, def("unit-b")).unwrap()),
            },
        );
    }
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let rules = RulePackageInput {
        existing_actor_rules: None,
        ordered_contributions: None,
        effect_applications: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("test"),
        semantics_version: key("test"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: f.owners,
        tables: vec![],
        receivers: empty(),
    };
    let error = CompiledRulePackage::compile(&rules, &schema, RuleLimits::default()).unwrap_err();
    assert!(error.to_string().contains("type/unit"), "{error}");
}
