//! Lookup domain evidence remains typed across occurrence-bound consumers.
#[allow(dead_code)] // Other targets use the remaining shared construction helpers.
#[path = "support/owned_plan_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, RoutingLimits},
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
};
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{CompiledRulePackage, RuleLimits},
};
use std::sync::Arc;

fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn bounded(n: i64) -> BoundedInteger {
    BoundedInteger::new(n).unwrap()
}
fn supplied_slot() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("supplied-skill"),
    }
}
fn supplied_grant() -> DeclaredSlot<GrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(def("summoner")),
        slot: def("supplied-grant"),
    }
}
fn required() -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Skill(def("skill")), "required-table-value")
}
fn selected(use_id: u64) -> ActionSelection {
    let mut a = action();
    a.action.provider = summoner_provider(use_id);
    a.action.provider.grant_path.push(supplied_grant());
    a
}
fn actor_stat(use_id: u64, name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Actor(child_actor(use_id)),
        stat: def(name),
    }
}
fn action_stat(use_id: u64, name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity: ConcreteEntity::Action(Box::new(selected(use_id))),
        stat: def(name),
    }
}
fn required_value(use_id: u64) -> PlanValueKey {
    PlanValueKey::SkillParameter {
        skill: Box::new(GeneratedSkillKey {
            provider: summoner_provider(use_id),
            slot: supplied_slot(),
        }),
        parameter: required(),
    }
}
fn lookup(node_id: &str, table: &str) -> RuleNode {
    node(
        node_id,
        RuleExpression::LookupIntegerTable {
            table: key(table),
            key: key("level"),
        },
    )
}
fn domain(node: &str, table: &str, requested: i64) -> EffectValue {
    EffectValue::UnsupportedDomain {
        node: key(node),
        table: key(table),
        key: bounded(requested),
        minimum: bounded(20),
        maximum: bounded(20),
    }
}
fn table(id: &str, value_type: ComputedValueType, row: ParameterValue) -> IntegerRuleTable {
    IntegerRuleTable {
        id: key(id),
        minimum: bounded(20),
        maximum: bounded(20),
        value_type,
        rows: vec![row],
    }
}
fn compile(f: &Fixture) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
    let definitions = Arc::new(
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap(),
    );
    let rules = RulePackageInput {
        receivers: DeclaredSet::complete(vec![]),
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("lookup-tests"),
        semantics_version: key("v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: definitions.identity().clone(),
        tables: vec![
            table("levels", ComputedValueType::Integer, integer(40)),
            table(
                "activation",
                ComputedValueType::Boolean,
                ParameterValue::Boolean(true),
            ),
        ],
        owners: f.owners.clone(),
    };
    let rules = Arc::new(
        CompiledRulePackage::compile(&rules, definitions.as_ref(), RuleLimits::default()).unwrap(),
    );
    let routes = Arc::new(
        OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("lookup-routes"),
                definitions: definitions.identity().clone(),
                outputs: f.routes.clone(),
            },
            definitions.as_ref(),
            RoutingLimits::default(),
        )
        .unwrap(),
    );
    OwnedEffectPlan::compile(
        Arc::new(f.request()),
        definitions,
        rules,
        routes,
        PlanLimits::default(),
    )
    .unwrap()
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let p = compile(f);
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn value<'a>(r: &'a OwnedEffectsReport, key: &PlanValueKey) -> &'a EffectValue {
    let mut rows = r.values.iter().filter(|v| &v.key == key);
    let row = rows
        .next()
        .unwrap_or_else(|| panic!("missing {key:?}: {r:?}"));
    assert!(rows.next().is_none());
    &row.value
}
fn supply(f: &mut Fixture) -> &mut RuleProgram {
    &mut f.owner_mut(&summoner_owner()).programs.members[0]
}
fn setup() -> Fixture {
    let mut f = Fixture::new();
    f.add_generated_actors();
    for definition in &mut f.schema.definitions {
        match definition {
            DefinitionDescriptor::Gem(e) if e.id == def("summoner") => {
                let SchemaState::Known(gem) = &mut e.schema else {
                    panic!()
                };
                gem.skills.members.push(def("skill"));
                gem.declarations.grants.members.push(supplied_grant());
                gem.declarations.skill_grants.members.push(supplied_slot());
            }
            DefinitionDescriptor::Skill(e) if e.id == def("skill") => {
                let SchemaState::Known(skill) = &mut e.schema else {
                    panic!()
                };
                skill.declarations.parameters.members.push(required());
            }
            _ => {}
        }
    }
    f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
        def("skill-constant"),
        StatSchema {
            value: ComputedValueType::Integer,
            targets: vec![RuleEntityKind::Action],
        },
    )));
    f.schema.slots.extend([
        SlotDescriptor::Parameter(entry(
            required(),
            ParameterSlotSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: bounded(0),
                    maximum: bounded(100),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )),
        SlotDescriptor::SkillGrant(entry(
            supplied_slot(),
            SkillGrantSlotSchema {
                skill: def("skill"),
                outputs: DeclaredSet::complete(vec![output()]),
            },
        )),
        SlotDescriptor::Grant(entry(
            supplied_grant(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(supplied_slot()),
            },
        )),
    ]);
    let program = supply(&mut f);
    program.nodes.push(lookup("lookup-level", "levels"));
    for effect in &mut program.effects {
        if let RuleEffectKind::ProjectActorStat { value, .. } = &mut effect.effect {
            *value = key("lookup-level");
        }
    }
    program.effects.extend([
        effect(
            "required-value",
            RuleEffectKind::ProjectSkillParameter {
                skill: supplied_slot(),
                parameter: required(),
                value: key("lookup-level"),
            },
        ),
        effect(
            "activate-skill",
            RuleEffectKind::ActivateGrant {
                slot: supplied_grant(),
                enabled: key("enabled"),
            },
        ),
    ]);
    f.owner_mut(&subject(def::<SkillDefinition>("skill")))
        .programs
        .members
        .push(RuleProgram {
            id: key("constant-consumer"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![literal("constant", 77)],
            effects: vec![derive(
                "constant",
                RuleEntity::Current,
                "skill-constant",
                "constant",
            )],
        });
    f.routes.push(ActionOutputRoutes {
        output: output(),
        routes: DeclaredSet::complete(vec![ActionStatRoute {
            id: key("actor-route"),
            selection: ActionRouteSelection::All,
            source: ActionStatRouteSource::ActionActor {
                stat: def("actor-total"),
            },
            target: def("routed"),
        }]),
    });
    for use_id in [30, 31] {
        f.queries.requests.push(MetricRequest {
            id: QueryId::new(format!("lookup-{use_id}")).unwrap(),
            metric: def("requested"),
            target: MetricTarget::Action(Box::new(selected(use_id))),
        });
    }
    f
}

#[test]
fn domain_cause_survives_actor_reads_required_skill_readiness_and_action_routes() {
    let r = evaluate(&setup());
    assert!(r.gaps.is_empty(), "{r:?}");
    let expected = domain("lookup-level", "levels", 11);
    for key in [
        actor_stat(30, "child-level"),
        actor_stat(30, "child-plus"),
        required_value(30),
        action_stat(30, "skill-constant"),
        action_stat(30, "routed"),
    ] {
        assert_eq!(value(&r, &key), &expected, "{key:?}");
    }
    for (key, n) in [
        (actor_stat(31, "child-level"), 40),
        (actor_stat(31, "child-plus"), 41),
        (required_value(31), 40),
        (action_stat(31, "skill-constant"), 77),
        (action_stat(31, "routed"), 16),
    ] {
        assert_eq!(value(&r, &key), &EffectValue::Known { value: integer(n) });
    }
    let diagnostic = serde_json::to_value(&expected).unwrap();
    assert_eq!(diagnostic["status"], "unsupported_domain");
    assert_eq!(diagnostic["node"], "lookup-level");
    assert_eq!(diagnostic["table"], "levels");
    assert_eq!(diagnostic["key"], 11);
    assert_eq!(diagnostic["minimum"], 20);
    assert_eq!(diagnostic["maximum"], 20);
}

#[test]
fn false_grants_deactivate_children_without_erasing_the_parent_domain_evidence() {
    let mut f = setup();
    f.build.gems[0].parameters[0].value = ParameterValue::Boolean(false);
    let r = evaluate(&f);
    assert_eq!(
        value(&r, &required_value(30)),
        &domain("lookup-level", "levels", 11)
    );
    assert_eq!(
        value(&r, &actor_stat(30, "child-level")),
        &domain("lookup-level", "levels", 11)
    );
    for key in [
        actor_stat(30, "child-plus"),
        action_stat(30, "skill-constant"),
        action_stat(30, "routed"),
    ] {
        assert_eq!(value(&r, &key), &EffectValue::Inactive);
    }
}

#[test]
fn an_unsupported_boolean_lookup_is_not_a_false_grant_or_missing_activation() {
    let mut f = setup();
    let program = supply(&mut f);
    program
        .nodes
        .push(lookup("activation-domain", "activation"));
    for effect in &mut program.effects {
        match &mut effect.effect {
            RuleEffectKind::ProjectActorStat { value, .. }
            | RuleEffectKind::ProjectSkillParameter { value, .. } => *value = key("level"),
            RuleEffectKind::ActivateGrant { enabled, .. } => *enabled = key("activation-domain"),
            _ => {}
        }
    }
    let r = evaluate(&f);
    assert!(r.gaps.is_empty(), "{r:?}");
    assert_eq!(
        value(&r, &required_value(30)),
        &EffectValue::Known { value: integer(11) }
    );
    assert_eq!(
        value(&r, &actor_stat(30, "child-level")),
        &EffectValue::Known { value: integer(11) }
    );
    for key in [
        actor_stat(30, "child-plus"),
        action_stat(30, "skill-constant"),
        action_stat(30, "routed"),
        PlanValueKey::Grant {
            provider: summoner_provider(30),
            slot: supplied_grant(),
        },
    ] {
        assert_eq!(
            value(&r, &key),
            &domain("activation-domain", "activation", 11)
        );
    }
}

#[test]
fn an_undemanded_upstream_domain_failure_does_not_poison_a_lazy_consumer() {
    let mut f = setup();
    let program = &mut f
        .owner_mut(&SchemaSubject::Slot(SlotAddress::Actor(child_slot())))
        .programs
        .members[0];
    program.nodes.extend([
        node(
            "choose",
            RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        ),
        literal("fallback", 7),
        node(
            "lazy",
            RuleExpression::Select {
                condition: key("choose"),
                when_true: key("plus"),
                when_false: key("fallback"),
            },
        ),
    ]);
    let RuleEffectKind::Derive { value: source, .. } = &mut program.effects[0].effect else {
        panic!()
    };
    *source = key("lazy");
    let r = evaluate(&f);
    assert_eq!(
        value(&r, &actor_stat(30, "child-level")),
        &domain("lookup-level", "levels", 11)
    );
    assert_eq!(
        value(&r, &actor_stat(30, "child-plus")),
        &EffectValue::Known { value: integer(7) }
    );
}

#[test]
fn partial_contributor_closure_withholds_finals_but_retains_the_lookup_diagnostic() {
    let mut f = setup();
    f.owner_mut(&class_owner()).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: class_owner(),
            facet: SchemaFacet::GameRules,
            code: key("unknown-class-programs"),
        }],
    };
    let r = evaluate(&f);
    assert!(
        r.gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms)
    );
    let target = required_value(30);
    let original = r
        .effects
        .iter()
        .find(|effect| {
            effect.target
                == BoundEffectTarget::Value {
                    key: target.clone(),
                }
        })
        .unwrap();
    assert_eq!(original.value, domain("lookup-level", "levels", 11));
    assert_eq!(
        value(&r, &target),
        &EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            read: None
        }
    );
}

#[test]
fn plan_scratch_reuse_cannot_turn_a_prior_valid_row_into_an_out_of_domain_value() {
    let a = setup();
    let mut b = setup();
    b.build.gems[0].level = 20;
    let pa = compile(&a);
    let pb = compile(&b);
    assert_ne!(pa.bindings().request, pb.bindings().request);
    let mut scratch = pa.new_scratch();
    for (plan, expected) in [
        (&pa, domain("lookup-level", "levels", 11)),
        (&pb, EffectValue::Known { value: integer(40) }),
        (&pa, domain("lookup-level", "levels", 11)),
    ] {
        let r = plan.evaluate(&mut scratch).unwrap();
        assert_eq!(value(&r, &required_value(30)), &expected);
    }
}
