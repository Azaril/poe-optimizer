//! Synthetic mechanics exercise Scenario expansion, independently of game data.
use super::*;

fn allow(f: &mut Fixture, members: Vec<UsageSkillApplicability>) {
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::UsagePolicy(entry) = descriptor
            && entry.id == def("skill-usage")
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.scenario_skill_applicability = Some(DeclaredSet::complete(members));
            return;
        }
    }
    panic!("fixture policy");
}
fn direct() -> UsageSkillApplicability {
    UsageSkillApplicability::AuthoredDirect {
        skill: def("skill"),
    }
}
fn supplied() -> UsageSkillApplicability {
    UsageSkillApplicability::Supplied(Box::new(UsageSkillSupply {
        skill: def("skill"),
        grant: skill_activation(),
        supply: skill_supply(),
    }))
}
fn intent(f: &mut Fixture, enabled: bool) {
    f.scenario.usage_intents = vec![ScenarioUsageIntent {
        policy: def("skill-usage"),
        parameters: usage("skill-usage", authored(20), enabled).parameters,
    }];
}
fn census(plan: &OwnedEffectPlan<OwnedDefinitionSchemaPackage>) -> &ResolvedUsageIntent {
    &plan.usage_resolution().intents[0]
}
fn partial(subject: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject,
            facet: SchemaFacet::InputSchema,
            code: key("unreviewed"),
        }],
    }
}

#[test]
fn scenario_expands_repeated_direct_skills_without_creating_actions() {
    let mut f = base();
    allow(&mut f, vec![direct()]);
    intent(&mut f, true);
    let plan = f.compile().unwrap();
    assert!(census(&plan).complete, "{:?}", plan.usage_resolution());
    assert_eq!(census(&plan).matches.len(), 2);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{report:?}");
    known(&report, skill_entity(20), true);
    known(&report, skill_entity(21), true);
    assert!(!report.values.iter().any(|value| {
        matches!(
            &value.key,
            PlanValueKey::Stat {
                entity: ConcreteEntity::Action(_),
                ..
            }
        )
    }));
    assert!(plan.usage_resolution().selections.iter().all(|selection| {
        selection.source == UsageSelectionSource::ScenarioIntent { index: 0 }
    }));
}

#[test]
fn whole_record_precedence_preserves_sources_and_removes_absent_parameters() {
    let mut f = base();
    allow(&mut f, vec![direct()]);
    intent(&mut f, true);
    f.build.usage_preferences = vec![usage("skill-usage", authored(20), false)];
    f.scenario.usage = vec![usage("skill-usage", authored(20), false)];
    let plan = f.compile().unwrap();
    let row = plan
        .usage_resolution()
        .selections
        .iter()
        .find(|row| row.selection.target == authored(20))
        .unwrap();
    assert_eq!(row.source, UsageSelectionSource::ExactScenario { index: 0 });
    assert_eq!(
        row.replaced,
        vec![
            UsageSelectionSource::Preset { index: 0 },
            UsageSelectionSource::ScenarioIntent { index: 0 },
        ]
    );
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    known(&report, skill_entity(20), false);
    known(&report, skill_entity(21), true);
    f.scenario.usage[0].parameters.clear();
    let missing = evaluate(&f);
    assert!(matches!(
        result(&missing, &skill_entity(20)),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingInput,
            ..
        }
    ));
    known(&missing, skill_entity(21), true);
    // An overridden lower record is still independently schema-checked.
    f.build.usage_preferences[0].parameters[0].value = integer(9);
    assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
}

#[test]
fn complete_empty_and_numerical_gaps_are_distinct_from_incomplete_census() {
    let mut f = base();
    allow(&mut f, vec![]);
    intent(&mut f, true);
    let dormant = f.compile().unwrap();
    assert!(census(&dormant).complete);
    assert!(census(&dormant).matches.is_empty());
    assert!(dormant.usage_resolution().selections.is_empty());
    assert_eq!(
        activation_effects(&dormant.evaluate(&mut dormant.new_scratch()).unwrap()),
        0
    );
    // Dormant parameters cannot hide malformed values.
    f.scenario.usage_intents[0].parameters[0].value = integer(1);
    assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
    intent(&mut f, true);
    allow(&mut f, vec![direct()]);
    f.owner_mut(&policy_owner("skill-usage")).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: policy_owner("skill-usage"),
            facet: SchemaFacet::GameRules,
            code: key("unreviewed-programs"),
        }],
    };
    let numerical = f.compile().unwrap();
    assert!(census(&numerical).complete);
    assert!(
        numerical
            .gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::UsagePolicy(entry) = descriptor
            && entry.id == def("skill-usage")
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema
                .scenario_skill_applicability
                .as_mut()
                .unwrap()
                .closure = partial(policy_owner("skill-usage"));
        }
    }
    f.scenario.usage = vec![usage("skill-usage", authored(20), false)];
    let unknown = f.compile().unwrap();
    assert!(!census(&unknown).complete);
    assert!(
        unknown
            .gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::UnresolvedTopology)
    );
}

#[test]
fn generated_relations_are_exact_and_disabled_or_other_loadout_roots_remain_in_census() {
    for disabled in [false, true] {
        let mut f = generated_fixture();
        allow(&mut f, vec![supplied()]);
        intent(&mut f, true);
        let root = f
            .build
            .skills
            .iter_mut()
            .find(|s| s.id == occurrence(31))
            .unwrap();
        if disabled {
            root.enabled = false;
        } else {
            root.scope = LoadoutScope::Selected {
                loadouts: vec![occurrence(2)],
            };
        }
        let plan = f.compile().unwrap();
        assert!(census(&plan).complete, "{:?}", plan.usage_resolution());
        assert_eq!(census(&plan).matches.len(), 2);
        for row in &census(&plan).matches {
            assert_eq!(row.provider.grant_path, vec![skill_activation()]);
            assert!(matches!(row.target, SkillTarget::Generated(_)));
        }
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        known(
            &report,
            ConcreteEntity::Skill(Box::new(generated(30))),
            true,
        );
        assert!(!report.values.iter().any(|value| {
            matches!(&value.key, PlanValueKey::Stat { entity: ConcreteEntity::Skill(target), stat }
                if **target == generated(31) && *stat == def::<StatDefinition>("effect-enabled"))
        }));
        assert!(!report.values.iter().any(|value| {
            matches!(&value.key, PlanValueKey::Stat { entity: ConcreteEntity::Skill(target), stat }
                if **target == SkillTarget::Authored(occurrence(30)) && *stat == def::<StatDefinition>("effect-enabled"))
        }));
    }
}

#[test]
fn partial_inactive_topology_is_unknown_and_never_hidden_by_override() {
    let mut f = generated_fixture();
    allow(&mut f, vec![supplied()]);
    intent(&mut f, true);
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(31))
        .unwrap()
        .enabled = false;
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::Gem(entry) = descriptor
            && entry.id == def("summoner")
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.declarations.grants.closure = partial(summoner_owner());
        }
    }
    f.scenario.usage = vec![usage(
        "skill-usage",
        UsageTarget::Skill(generated(30)),
        false,
    )];
    let plan = f.compile().unwrap();
    assert!(!census(&plan).complete);
    assert!(census(&plan).gaps.iter().any(|gap| {
        gap.reason == PlanGapReason::PartialDeclarations
            && gap
                .provider
                .as_ref()
                .is_some_and(|p| p.root == ProviderRoot::SkillUse(occurrence(31)))
    }));
    assert_eq!(plan.usage_resolution().selections.len(), 2);
}

#[test]
fn item_sources_expand_to_exact_equipment_uses() {
    let original = Fixture::new();
    let mut f = base();
    f.build.items = original.build.items;
    f.build.equipment = original.build.equipment;
    let supply = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(def("item")),
        slot: def::<SkillGrantSlotDefinition>("item-skill"),
    };
    let grant = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(def("item")),
        slot: def::<GrantSlotDefinition>("item-skill-grant"),
    };
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(entry) = descriptor
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema
                .declarations
                .skill_grants
                .members
                .push(supply.clone());
            schema.declarations.grants.members.push(grant.clone());
        }
    }
    f.schema.slots.extend([
        SlotDescriptor::SkillGrant(entry(
            supply.clone(),
            SkillGrantSlotSchema {
                preset_inputs: None,
                skill: def("skill"),
                outputs: DeclaredSet::complete(vec![output()]),
            },
        )),
        SlotDescriptor::Grant(entry(
            grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::EquipmentUse],
                target: GrantTarget::Skill(supply.clone()),
            },
        )),
    ]);
    let program = &mut f.owner_mut(&item_owner()).programs.members[0];
    program.nodes.push(node(
        "enabled",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(true),
        },
    ));
    program.effects.push(effect(
        "supply-skill",
        RuleEffectKind::ActivateGrant {
            slot: grant.clone(),
            enabled: key("enabled"),
        },
    ));
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(GrantSlotDefId::address(&grant)),
        programs: DeclaredSet::complete(vec![]),
    });
    allow(
        &mut f,
        vec![UsageSkillApplicability::Supplied(Box::new(
            UsageSkillSupply {
                skill: def("skill"),
                grant: grant.clone(),
                supply: supply.clone(),
            },
        ))],
    );
    intent(&mut f, true);
    let plan = f.compile().unwrap();
    assert!(census(&plan).complete, "{:?}", plan.usage_resolution());
    assert_eq!(census(&plan).matches.len(), 2);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for id in [6, 7] {
        let target = SkillTarget::Generated(Box::new(GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::EquipmentUse(occurrence(id)),
                grant_path: vec![],
            },
            slot: supply.clone(),
        }));
        known(&report, ConcreteEntity::Skill(Box::new(target)), true);
    }
}

#[test]
fn intent_values_and_provenance_are_deterministic_across_reused_and_parallel_workers() {
    let mut f = generated_fixture();
    allow(&mut f, vec![direct(), supplied()]);
    intent(&mut f, true);
    let a = f.compile().unwrap();
    let rebuilt = f.compile().unwrap();
    assert_eq!(a.identity(), rebuilt.identity());
    assert_eq!(a.usage_resolution(), rebuilt.usage_resolution());
    intent(&mut f, false);
    let b = f.compile().unwrap();
    assert_ne!(a.identity(), b.identity());
    let mut scratch = a.new_scratch();
    let expected = a.evaluate(&mut scratch).unwrap();
    known(&b.evaluate(&mut scratch).unwrap(), skill_entity(20), false);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(reports.iter().all(|report| report == &expected));
    assert!(matches!(
        f.compile_with(PlanLimits {
            max_owner_bindings: 1,
            ..PlanLimits::default()
        }),
        Err(PlanError::Limit(_))
    ));
}

#[test]
fn unknown_implicit_passive_owner_cannot_certify_dormant_intent() {
    let mut f = base();
    allow(&mut f, vec![direct()]);
    intent(&mut f, true);
    f.build.skills.clear();
    let passive = def::<PassiveNodeDefinition>("unmapped-root");
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::Class(entry) = descriptor
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.implicit_passives.members.push(passive.clone());
        }
    }
    f.schema
        .definitions
        .push(DefinitionDescriptor::PassiveNode(DefinitionEntry {
            id: passive.clone(),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(passive.address()),
                    facet: SchemaFacet::InputSchema,
                    code: key("unmapped-root"),
                }],
            },
        }));
    let plan = f.compile().unwrap();
    assert!(!census(&plan).complete);
    assert!(census(&plan).matches.is_empty());
    assert!(census(&plan).gaps.iter().any(|gap| {
        gap.subject == Some(SchemaSubject::Definition(passive.address()))
            && gap.reason == PlanGapReason::UnresolvedTopology
    }));
}

struct PolicyIndex {
    base: OwnedDefinitionSchemaPackage,
    policy: DefinitionDescriptor,
}
impl DefinitionSchemaIndex for PolicyIndex {
    fn identity(&self) -> &poe_optimizer_core::data::DataIdentity {
        self.base.identity()
    }
    fn namespace(&self) -> &GameVersionNamespace {
        self.base.namespace()
    }
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        if *address == self.policy.address() {
            Some(&self.policy)
        } else {
            self.base.lookup_definition(address)
        }
    }
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        self.base.lookup_slot(address)
    }
}

#[test]
fn custom_index_unknown_allowance_does_not_hide_later_known_contradiction() {
    let mut f = generated_fixture();
    allow(&mut f, vec![supplied()]);
    intent(&mut f, true);
    let base =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let mut policy = base
        .lookup_definition(&def::<UsagePolicyDefinition>("skill-usage").address())
        .unwrap()
        .clone();
    if let DefinitionDescriptor::UsagePolicy(entry) = &mut policy
        && let SchemaState::Known(schema) = &mut entry.schema
    {
        schema.scenario_skill_applicability = Some(DeclaredSet::complete(vec![
            UsageSkillApplicability::AuthoredDirect {
                skill: def("absent"),
            },
            UsageSkillApplicability::Supplied(Box::new(UsageSkillSupply {
                skill: def("skill"),
                grant: child_grant(),
                supply: skill_supply(),
            })),
        ]));
    }
    let result = f.compile_with_index(
        Arc::new(PolicyIndex { base, policy }),
        PlanLimits::default(),
        OWNED_RULE_OPERATIONS_V14,
    );
    assert!(
        matches!(result, Err(PlanError::Invalid(message)) if message == "Scenario usage grant has a different target")
    );
}

#[test]
fn expanded_census_diagnostics_are_bounded_before_copying_each_intent() {
    let mut f = base();
    for name in ["intent-b", "intent-c", "intent-d"] {
        add_policy(&mut f, name, UsageTargetKind::Skill, RuleEntityKind::Skill);
    }
    for descriptor in &mut f.schema.definitions {
        match descriptor {
            DefinitionDescriptor::UsagePolicy(entry)
                if entry.id != def("action-usage") && entry.id != def("actor-usage") =>
            {
                if let SchemaState::Known(schema) = &mut entry.schema {
                    schema.scenario_skill_applicability = Some(DeclaredSet::complete(vec![]));
                }
            }
            DefinitionDescriptor::Class(entry) => {
                if let SchemaState::Known(schema) = &mut entry.schema {
                    schema.declarations.grants.closure = partial(class_owner());
                }
            }
            DefinitionDescriptor::Skill(entry) => {
                if let SchemaState::Known(schema) = &mut entry.schema {
                    schema.declarations.grants.closure =
                        partial(subject(def::<SkillDefinition>("skill")));
                }
            }
            _ => {}
        }
    }
    f.scenario.usage_intents = ["skill-usage", "intent-b", "intent-c", "intent-d"]
        .into_iter()
        .map(|name| ScenarioUsageIntent {
            policy: def(name),
            parameters: usage(name, authored(20), true).parameters,
        })
        .collect();
    assert!(matches!(
        f.compile_with(PlanLimits {
            max_owner_bindings: 8,
            ..PlanLimits::default()
        }),
        Err(PlanError::Limit("usage resolution diagnostics"))
    ));
}

#[test]
fn disabled_second_supply_path_is_ambiguous_even_when_not_allowlisted_and_in_either_order() {
    let alternate = DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def::<GrantSlotDefinition>("second-entering-grant"),
    };
    let mut ordered = [skill_activation(), alternate.clone()];
    ordered.sort();
    for admitted in &ordered {
        let mut f = generated_fixture();
        for skill in &mut f.build.skills {
            if matches!(skill.source, AuthoredSkillSource::Gem(_)) {
                skill.enabled = false;
            }
        }
        for descriptor in &mut f.schema.definitions {
            if let DefinitionDescriptor::Gem(entry) = descriptor
                && entry.id == def("summoner")
                && let SchemaState::Known(schema) = &mut entry.schema
            {
                schema.declarations.grants.members.push(alternate.clone());
            }
        }
        f.schema.slots.push(SlotDescriptor::Grant(entry(
            alternate.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(skill_supply()),
            },
        )));
        f.owners.push(DefinitionRules {
            owner: SchemaSubject::Slot(GrantSlotDefId::address(&alternate)),
            programs: DeclaredSet::complete(vec![]),
        });
        allow(
            &mut f,
            vec![UsageSkillApplicability::Supplied(Box::new(
                UsageSkillSupply {
                    skill: def("skill"),
                    grant: admitted.clone(),
                    supply: skill_supply(),
                },
            ))],
        );
        intent(&mut f, true);
        // Choosing either sorted grant proves the disallowed path is seen both
        // before and after the allowed one; neither ordering can hide ambiguity.
        assert!(matches!(f.compile(), Err(PlanError::Invalid(message))
            if message == "ambiguous Scenario usage supplying relation"));
    }
}
