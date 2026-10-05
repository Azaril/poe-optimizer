//! Native usage programs consume the ordinary composed request and exact targets.
//! Synthetic Boolean effect activation proves the seam, not game/build coverage.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_project::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, RoutingLimits},
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::*};
use rayon::prelude::*;
use std::sync::Arc;

fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn ports() -> DeclaredSlots {
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
fn policy_owner(name: &str) -> SchemaSubject {
    subject(def::<UsagePolicyDefinition>(name))
}
fn policy_input(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::UsagePolicy(def(name)), "enabled")
}
fn add_policy(f: &mut Fixture, name: &str, target: UsageTargetKind, context: RuleEntityKind) {
    let mut declarations = ports();
    declarations.parameters.members.push(policy_input(name));
    f.schema
        .definitions
        .push(DefinitionDescriptor::UsagePolicy(entry(
            def(name),
            UsagePolicySchema {
                targets: vec![target],
                declarations,
            },
        )));
    f.schema.slots.push(SlotDescriptor::Parameter(entry(
        policy_input(name),
        ParameterSlotSchema {
            skill_input: None,
            value: ValueSchema::Boolean,
            presence: SlotPresence::OptionalOnce,
            sites: vec![ParameterSite::UsagePolicyParameter],
        },
    )));
    f.owners.push(DefinitionRules {
        owner: policy_owner(name),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("usage-activation"),
            context,
            reads: vec![RuleRead {
                id: key("enabled"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Parameter {
                    slot: policy_input(name),
                },
            }],
            nodes: vec![read_node("enabled", "enabled")],
            effects: vec![derive(
                "activation",
                RuleEntity::Current,
                "effect-enabled",
                "enabled",
            )],
        }]),
    });
}
fn usage(name: &str, target: UsageTarget, enabled: bool) -> UsagePolicySelection {
    UsagePolicySelection {
        policy: def(name),
        target,
        parameters: vec![ParameterAssignment {
            slot: policy_input(name),
            value: ParameterValue::Boolean(enabled),
        }],
    }
}
fn authored(id: u64) -> UsageTarget {
    UsageTarget::Skill(SkillTarget::Authored(occurrence(id)))
}
fn skill_entity(id: u64) -> ConcreteEntity {
    ConcreteEntity::Skill(Box::new(SkillTarget::Authored(occurrence(id))))
}
fn selected_action(id: u64) -> ActionSelection {
    let mut selected = action();
    selected.action.provider.root = ProviderRoot::SkillUse(occurrence(id));
    selected
}
fn base() -> Fixture {
    let mut f = Fixture::new();
    f.build.items.clear();
    f.build.equipment.clear();
    f.owner_mut(&class_owner()).programs.members.clear();
    f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
        def("effect-enabled"),
        StatSchema {
            value: ComputedValueType::Boolean,
            targets: vec![
                RuleEntityKind::Skill,
                RuleEntityKind::Action,
                RuleEntityKind::Actor,
            ],
        },
    )));
    for id in [20, 21] {
        f.build.skills.push(SkillUse {
            parameters: None,
            id: occurrence(id),
            source: AuthoredSkillSource::Direct(def("skill")),
            enabled: true,
            scope: LoadoutScope::Shared,
        });
    }
    f.routes.push(ActionOutputRoutes {
        source_selectors: Some(DeclaredSet::complete(vec![])),
        output: output(),
        routes: DeclaredSet::complete(vec![]),
    });
    add_policy(
        &mut f,
        "skill-usage",
        UsageTargetKind::Skill,
        RuleEntityKind::Skill,
    );
    add_policy(
        &mut f,
        "action-usage",
        UsageTargetKind::Action,
        RuleEntityKind::Action,
    );
    add_policy(
        &mut f,
        "actor-usage",
        UsageTargetKind::Actor,
        RuleEntityKind::Actor,
    );
    f
}
fn result<'a>(report: &'a OwnedEffectsReport, entity: &ConcreteEntity) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|r| {
            r.key
                == PlanValueKey::Stat {
                    entity: entity.clone(),
                    stat: def("effect-enabled"),
                }
        })
        .unwrap_or_else(|| panic!("missing {entity:?}: {report:?}"))
        .value
}
fn known(report: &OwnedEffectsReport, entity: ConcreteEntity, enabled: bool) {
    assert_eq!(
        result(report, &entity),
        &EffectValue::Known {
            value: ParameterValue::Boolean(enabled)
        }
    );
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn activation_effects(report: &OwnedEffectsReport) -> usize {
    report
        .effects
        .iter()
        .filter(|e| matches!(e.key.invocation.origin, RuleOrigin::Usage { .. }))
        .count()
}

#[test]
fn explicit_policy_parameters_bind_only_the_exact_skill_action_or_actor() {
    let mut f = base();
    f.scenario.usage = vec![
        usage("skill-usage", authored(20), true),
        usage("skill-usage", authored(21), false),
        usage(
            "action-usage",
            UsageTarget::Action(Box::new(selected_action(20))),
            false,
        ),
        usage("actor-usage", UsageTarget::Actor(ActorKey::Player), true),
    ];
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{report:?}");
    assert_eq!(activation_effects(&report), 4);
    known(&report, skill_entity(20), true);
    known(&report, skill_entity(21), false);
    known(
        &report,
        ConcreteEntity::Action(Box::new(selected_action(20))),
        false,
    );
    known(&report, ConcreteEntity::Actor(ActorKey::Player), true);
    assert!(!report.values.iter().any(|r| matches!(&r.key, PlanValueKey::Stat {entity: ConcreteEntity::Action(a), ..} if **a == selected_action(21))));
    f.scenario.usage.clear();
    assert_eq!(activation_effects(&evaluate(&f)), 0);
}

#[test]
fn usage_does_not_enable_disabled_or_other_loadout_occurrences() {
    for disabled in [false, true] {
        let mut f = base();
        if disabled {
            f.build.skills[1].enabled = false;
        } else {
            f.build.skills[1].scope = LoadoutScope::Selected {
                loadouts: vec![occurrence(2)],
            };
        }
        f.scenario.usage = vec![
            usage("skill-usage", authored(20), false),
            usage("skill-usage", authored(21), true),
            usage(
                "action-usage",
                UsageTarget::Action(Box::new(selected_action(21))),
                true,
            ),
        ];
        let report = evaluate(&f);
        assert!(report.gaps.is_empty(), "{report:?}");
        assert_eq!(activation_effects(&report), 1);
        known(&report, skill_entity(20), false);
        if !disabled {
            f.build.active_weapon_loadout = occurrence(2);
            let active = evaluate(&f);
            assert!(active.gaps.is_empty(), "{active:?}");
            assert_eq!(activation_effects(&active), 3);
            known(&active, skill_entity(21), true);
        }
    }
}

fn skill_supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("usage-skill"),
    }
}
fn skill_activation() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("usage-skill-grant"),
    }
}
fn generated(id: u64) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: summoner_provider(id),
        slot: skill_supply(),
    }))
}
fn generated_action(id: u64) -> ActionSelection {
    let mut action = selected_action(id);
    action.action.provider.grant_path.push(skill_activation());
    action
}
fn projected_input() -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Skill(def("skill")), "projected-ready")
}
fn add_projected_input(f: &mut Fixture) {
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::Skill(entry) = descriptor
            && entry.id == def("skill")
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema
                .declarations
                .parameters
                .members
                .push(projected_input());
        }
    }
    f.schema.slots.push(SlotDescriptor::Parameter(entry(
        projected_input(),
        ParameterSlotSchema {
            skill_input: None,
            value: ValueSchema::Boolean,
            presence: SlotPresence::RequiredOnce,
            sites: vec![],
        },
    )));
}
fn generated_fixture() -> Fixture {
    let mut f = base();
    f.add_generated_actors();
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::Gem(entry) = descriptor
            && entry.id == def("summoner")
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.skills.members.push(def("skill"));
            schema
                .declarations
                .skill_grants
                .members
                .push(skill_supply());
            schema.declarations.grants.members.push(skill_activation());
        }
    }
    f.schema.slots.extend([
        SlotDescriptor::SkillGrant(entry(
            skill_supply(),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("skill"),
                outputs: DeclaredSet::complete(vec![output()]),
            },
        )),
        SlotDescriptor::Grant(entry(
            skill_activation(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(skill_supply()),
            },
        )),
    ]);
    f.owner_mut(&summoner_owner()).programs.members[0]
        .effects
        .push(effect(
            "supply-skill",
            RuleEffectKind::ActivateGrant {
                slot: skill_activation(),
                enabled: key("enabled"),
            },
        ));
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(GrantSlotDefId::address(&skill_activation())),
        programs: DeclaredSet::complete(vec![]),
    });
    f
}

#[test]
fn generated_targets_preserve_supply_activation_and_owned_actor_identity() {
    let mut f = generated_fixture();
    f.scenario.usage = vec![
        usage("skill-usage", UsageTarget::Skill(generated(30)), true),
        usage("skill-usage", UsageTarget::Skill(generated(31)), false),
        usage("actor-usage", UsageTarget::Actor(child_actor(30)), false),
        usage("actor-usage", UsageTarget::Actor(child_actor(31)), true),
    ];
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{report:?}");
    known(
        &report,
        ConcreteEntity::Skill(Box::new(generated(30))),
        true,
    );
    known(
        &report,
        ConcreteEntity::Skill(Box::new(generated(31))),
        false,
    );
    known(&report, ConcreteEntity::Actor(child_actor(30)), false);
    known(&report, ConcreteEntity::Actor(child_actor(31)), true);
    f.build.gems[0].parameters[0].value = ParameterValue::Boolean(false);
    let inactive = evaluate(&f);
    assert_eq!(
        result(&inactive, &ConcreteEntity::Skill(Box::new(generated(30)))),
        &EffectValue::Inactive
    );
    assert_eq!(
        result(&inactive, &ConcreteEntity::Actor(child_actor(30))),
        &EffectValue::Inactive
    );
    known(
        &inactive,
        ConcreteEntity::Skill(Box::new(generated(31))),
        false,
    );
    known(&inactive, ConcreteEntity::Actor(child_actor(31)), true);
}

#[test]
fn generated_actions_and_skills_retain_required_input_and_grant_gates() {
    let mut f = generated_fixture();
    f.scenario.usage = vec![
        usage(
            "action-usage",
            UsageTarget::Action(Box::new(generated_action(30))),
            true,
        ),
        usage(
            "action-usage",
            UsageTarget::Action(Box::new(generated_action(31))),
            false,
        ),
        usage("skill-usage", UsageTarget::Skill(generated(30)), true),
    ];
    let report = evaluate(&f);
    assert!(report.gaps.is_empty(), "{report:?}");
    known(
        &report,
        ConcreteEntity::Action(Box::new(generated_action(30))),
        true,
    );
    known(
        &report,
        ConcreteEntity::Action(Box::new(generated_action(31))),
        false,
    );
    add_projected_input(&mut f);
    let missing = evaluate(&f);
    for entity in [
        ConcreteEntity::Skill(Box::new(generated(30))),
        ConcreteEntity::Action(Box::new(generated_action(30))),
    ] {
        assert!(
            matches!(
                result(&missing, &entity),
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ),
            "{missing:?}"
        );
    }
    f.owner_mut(&summoner_owner()).programs.members[0]
        .effects
        .push(effect(
            "required-input",
            RuleEffectKind::ProjectSkillParameter {
                skill: skill_supply(),
                parameter: projected_input(),
                value: key("enabled"),
            },
        ));
    let supplied = evaluate(&f);
    known(
        &supplied,
        ConcreteEntity::Skill(Box::new(generated(30))),
        true,
    );
    known(
        &supplied,
        ConcreteEntity::Action(Box::new(generated_action(30))),
        true,
    );
    f.build.gems[0].parameters[0].value = ParameterValue::Boolean(false);
    let inactive = evaluate(&f);
    assert_eq!(
        result(
            &inactive,
            &ConcreteEntity::Action(Box::new(generated_action(30)))
        ),
        &EffectValue::Inactive
    );
    known(
        &inactive,
        ConcreteEntity::Action(Box::new(generated_action(31))),
        false,
    );
}

#[test]
fn usage_owned_child_slots_do_not_create_provider_topology_or_project_phantom_values() {
    let mut f = base();
    f.scenario.usage = vec![usage("skill-usage", authored(20), true)];
    let declaration = SlotOwnerDefId::UsagePolicy(def("skill-usage"));
    let actor = DeclaredSlot {
        declaration: declaration.clone(),
        slot: def::<ActorSlotDefinition>("policy-child"),
    };
    let skill = DeclaredSlot {
        declaration: declaration.clone(),
        slot: def::<SkillGrantSlotDefinition>("policy-skill"),
    };
    let grant = DeclaredSlot {
        declaration,
        slot: def::<GrantSlotDefinition>("policy-grant"),
    };
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::UsagePolicy(entry) = descriptor
            && entry.id == def("skill-usage")
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.declarations.actors.members.push(actor.clone());
            schema.declarations.skill_grants.members.push(skill.clone());
            schema.declarations.grants.members.push(grant.clone());
        }
    }
    f.schema.slots.extend([
        SlotDescriptor::Actor(entry(
            actor.clone(),
            ActorSlotSchema {
                provider_definition: None,
                skills: DeclaredSet::complete(vec![]),
                outputs: DeclaredSet::complete(vec![]),
            },
        )),
        SlotDescriptor::SkillGrant(entry(
            skill.clone(),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("skill"),
                outputs: DeclaredSet::complete(vec![output()]),
            },
        )),
        SlotDescriptor::Grant(entry(
            grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(skill.clone()),
            },
        )),
    ]);
    add_projected_input(&mut f);
    for operation in [
        RuleEffectKind::ActivateGrant {
            slot: grant,
            enabled: key("enabled"),
        },
        RuleEffectKind::ProjectActorStat {
            actor,
            stat: def("effect-enabled"),
            value: key("enabled"),
        },
        RuleEffectKind::ProjectSkillParameter {
            skill,
            parameter: projected_input(),
            value: key("enabled"),
        },
    ] {
        f.owner_mut(&policy_owner("skill-usage")).programs.members[0].effects =
            vec![effect("unsupported-topology", operation)];
        let report = evaluate(&f);
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::UnsupportedRelation),
            "{report:?}"
        );
        assert_eq!(activation_effects(&report), 0);
        assert!(!report.values.iter().any(|v| matches!(&v.key, PlanValueKey::Grant { slot, .. } if matches!(&slot.declaration, SlotOwnerDefId::UsagePolicy(_)))));
        assert!(!report.values.iter().any(|v| matches!(&v.key, PlanValueKey::SkillParameter { skill, .. } if matches!(&skill.slot.declaration, SlotOwnerDefId::UsagePolicy(_)))));
    }
}

#[test]
fn missing_inputs_and_incomplete_programs_never_become_known_usage_values() {
    let mut f = base();
    f.scenario.usage = vec![usage("skill-usage", authored(20), true)];
    f.scenario.usage[0].parameters.clear();
    let missing = evaluate(&f);
    assert!(
        matches!(
            result(&missing, &skill_entity(20)),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingInput,
                ..
            }
        ),
        "{missing:?}"
    );
    f.scenario.usage[0].parameters = usage("skill-usage", authored(20), true).parameters;
    let subject = policy_owner("skill-usage");
    f.owner_mut(&subject).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject.clone(),
            facet: SchemaFacet::GameRules,
            code: key("unknown-usage-programs"),
        }],
    };
    let partial = evaluate(&f);
    assert!(
        partial
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    assert!(
        matches!(
            result(&partial, &skill_entity(20)),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ),
        "{partial:?}"
    );
    f.owners.retain(|r| r.owner != subject);
    let missing = evaluate(&f);
    assert!(
        missing
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::MissingPrograms)
    );
    assert_eq!(activation_effects(&missing), 0);
}

#[test]
fn unsupported_cross_context_policy_programs_remain_explicit() {
    let mut f = base();
    f.scenario.usage = vec![usage("skill-usage", authored(20), true)];
    f.owner_mut(&policy_owner("skill-usage")).programs.members[0].context = RuleEntityKind::Actor;
    let report = evaluate(&f);
    assert_eq!(activation_effects(&report), 0);
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::UnsupportedContext)
    );
}

#[test]
fn missing_historical_targets_and_invalid_typed_values_do_not_execute() {
    let mut f = base();
    for target in [
        authored(22),
        UsageTarget::Action(Box::new(selected_action(22))),
    ] {
        let policy = if matches!(target, UsageTarget::Skill(_)) {
            "skill-usage"
        } else {
            "action-usage"
        };
        f.scenario.usage = vec![usage(policy, target, true)];
        // The complete request can preserve a historical selector; authored
        // schema binding rejects its missing provider rather than retargeting it.
        let _request = f.request();
        assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
    }
    f.scenario.usage = vec![usage("skill-usage", authored(20), true)];
    f.scenario.usage[0].parameters[0].value = integer(1);
    assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
    let mut f = generated_fixture();
    f.scenario.usage = vec![usage(
        "actor-usage",
        UsageTarget::Actor(child_actor(32)),
        true,
    )];
    let _request = f.request();
    assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
}

#[test]
fn partial_parameter_schema_keeps_normal_contributor_coverage_unresolved() {
    let mut f = base();
    f.scenario.usage = vec![usage("skill-usage", authored(20), true)];
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::UsagePolicy(entry) = descriptor
            && entry.id == def("skill-usage")
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.declarations.parameters.closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: policy_owner("skill-usage"),
                    facet: SchemaFacet::InputSchema,
                    code: key("unknown-parameters"),
                }],
            };
        }
    }
    let report = evaluate(&f);
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::SchemaUnresolved)
    );
    assert!(
        matches!(
            result(&report, &skill_entity(20)),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ),
        "{report:?}"
    );
}

#[test]
fn usage_obeys_plan_limits_and_worker_local_scratch_reuse() {
    let mut f = base();
    f.scenario.usage = vec![
        usage("skill-usage", authored(20), true),
        usage("skill-usage", authored(21), false),
    ];
    let limits = PlanLimits {
        max_invocations: 1,
        ..PlanLimits::default()
    };
    assert!(matches!(
        f.compile_with(limits),
        Err(PlanError::Limit("invocations"))
    ));
    let limits = PlanLimits {
        max_effects: 1,
        ..PlanLimits::default()
    };
    assert!(matches!(
        f.compile_with(limits),
        Err(PlanError::Limit("effects"))
    ));
    let a = f.compile().unwrap();
    f.scenario.usage[0].parameters[0].value = ParameterValue::Boolean(false);
    let b = f.compile().unwrap();
    let mut scratch = a.new_scratch();
    let expected = a.evaluate(&mut scratch).unwrap();
    known(&b.evaluate(&mut scratch).unwrap(), skill_entity(20), false);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    let reports: Vec<_> = (0..32)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(reports.iter().all(|r| r == &expected));
}

fn compile_request(
    f: &Fixture,
    request: OwnedEvaluationRequest,
) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
    let definitions = Arc::new(
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap(),
    );
    let rules = Arc::new(
        CompiledRulePackage::compile(
            &RulePackageInput {
                effect_applications: None,
                receivers: f.receivers.clone(),
                tables: f.tables.clone(),
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("rules"),
                semantics_version: key("test-v1"),
                operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
                definitions: definitions.identity().clone(),
                owners: f.owners.clone(),
            },
            definitions.as_ref(),
            RuleLimits::default(),
        )
        .unwrap(),
    );
    let routing = Arc::new(
        OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: definitions.identity().clone(),
                outputs: f.routes.clone(),
            },
            definitions.as_ref(),
            RoutingLimits::default(),
        )
        .unwrap(),
    );
    OwnedEffectPlan::compile(
        Arc::new(request),
        definitions,
        rules,
        routing,
        PlanLimits::default(),
    )
    .unwrap()
}

#[test]
fn composed_preferences_and_whole_record_overrides_execute_through_the_same_request() {
    let f = base();
    let b = &f.build;
    let limits = OwnedInputLimits::default();
    let selection = VariantSelection {
        character: occurrence(50),
        equipment: occurrence(51),
        allocations: occurrence(52),
        skills: occurrence(53),
        choices: occurrence(54),
        active_weapon_loadout: b.active_weapon_loadout,
    };
    let project = BuildProject::new(
        ProjectInput {
            allocator: b.allocator,
            revision: b.revision,
            game_version: b.game_version.clone(),
            weapon_loadouts: b.weapon_loadouts.clone(),
            items: b.items.clone(),
            gems: b.gems.clone(),
            rewards: b.character.rewards.clone(),
            equipment: b.equipment.clone(),
            allocations: b.allocations.clone(),
            skills: b.skills.clone(),
            supports: b.supports.clone(),
            payload_links: b.payload_links.clone(),
            character_presets: vec![CharacterPreset {
                id: selection.character,
                class: b.character.class.clone(),
                ascendancy: b.character.ascendancy.clone(),
                level: b.character.level,
                rewards: vec![],
            }],
            equipment_presets: vec![EquipmentPreset {
                id: selection.equipment,
                equipment: vec![],
            }],
            allocation_presets: vec![AllocationPreset {
                id: selection.allocations,
                allocations: vec![],
                equipment: vec![],
            }],
            skill_presets: vec![SkillPreset {
                intent: None,
                id: selection.skills,
                skills: b.skills.iter().map(|s| s.id).collect(),
                supports: vec![],
                support_origins: None,
                payload_links: vec![],
                usage_preferences: Some(vec![
                    usage("skill-usage", authored(20), true),
                    usage("skill-usage", authored(21), false),
                ]),
            }],
            choice_presets: vec![ChoicePreset {
                id: selection.choices,
                choices: vec![],
                rewards: vec![],
            }],
            saved_variants: vec![],
        },
        limits,
    )
    .unwrap();
    let compose = |scenario: ScenarioInput| {
        compose_request(
            &project,
            &selection,
            None,
            ScenarioSpec::new(scenario, limits).unwrap(),
            QuerySpec::new(f.queries.clone(), limits).unwrap(),
            limits,
        )
        .unwrap()
    };
    let request = compose(f.scenario.clone());
    let plan = compile_request(&f, request);
    known(
        &plan.evaluate(&mut plan.new_scratch()).unwrap(),
        skill_entity(20),
        true,
    );
    let mut scenario = f.scenario.clone();
    scenario.usage = vec![usage("skill-usage", authored(20), false)];
    let plan = compile_request(&f, compose(scenario.clone()));
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    known(&report, skill_entity(20), false);
    known(&report, skill_entity(21), false);
    // The empty overriding record removes the preferred value; no field merge
    // or default fabricates a Boolean for the native consumer.
    scenario.usage[0].parameters.clear();
    let plan = compile_request(&f, compose(scenario));
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        result(&report, &skill_entity(20)),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingInput,
            ..
        }
    ));
    known(&report, skill_entity(21), false);
}
