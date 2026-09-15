//! Regression guards for bounded binding and potential/unsupported providers.
#[path = "support/owned_plan_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;

fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn declarations() -> DeclaredSlots {
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
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn stat(entity: ConcreteEntity, name: &str) -> PlanValueKey {
    PlanValueKey::Stat {
        entity,
        stat: def(name),
    }
}
fn evaluate(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn assert_withheld(report: &OwnedEffectsReport, key: &PlanValueKey) {
    let rows = report
        .values
        .iter()
        .filter(|row| &row.key == key)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "expected one final value for {key:?}");
    assert!(
        matches!(
            rows[0].value,
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ),
        "final value was not withheld: {:?}",
        rows[0].value
    );
}
#[test]
fn unused_contribution_reads_cannot_bypass_the_binding_edge_budget() {
    let mut f = Fixture::new();
    f.add_level_program();
    let limits = PlanLimits {
        max_edges: 64,
        ..PlanLimits::default()
    };
    assert!(
        f.compile_with(limits).is_ok(),
        "positive control graph fits the budget"
    );
    let program = &mut f.owner_mut(&item_owner()).programs.members[0];
    let nodes_before = program.nodes.clone();
    let effects_before = program.effects.clone();
    for ordinal in 0..64 {
        program.reads.push(contributions(
            &format!("unused-{ordinal}"),
            RuleEntity::Current,
            "local",
        ));
    }
    // These reads have no expression nodes and are never statically demanded.
    // Binding still expands each read to the complete contributor set per use.
    assert_eq!(program.nodes, nodes_before);
    assert_eq!(program.effects, effects_before);
    let error = f
        .compile_with(limits)
        .err()
        .expect("unused read expansion must be bounded before cloning");
    assert!(
        matches!(error, PlanError::Limit(name) if name.contains("edges")),
        "{error}"
    );
    assert!(
        f.compile().is_ok(),
        "the package is otherwise valid under the normal bound"
    );
}
#[test]
fn potential_gem_skill_membership_does_not_activate_actor_contributions() {
    let mut f = Fixture::new();
    f.add_generated_actors();
    let DefinitionDescriptor::Gem(gem) = f
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == def::<GemDefinition>("summoner").address())
        .unwrap()
    else {
        panic!()
    };
    let SchemaState::Known(gem) = &mut gem.schema else {
        panic!()
    };
    gem.skills.members.push(def("skill"));
    let skill_owner = subject(def::<SkillDefinition>("skill"));
    f.owner_mut(&skill_owner)
        .programs
        .members
        .push(RuleProgram {
            id: key("unselected-skill-bonus"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![literal("bonus", 999)],
            effects: vec![effect(
                "bonus",
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Actor,
                    stat: def("actor-total"),
                    contribution: ContributionKind::Add,
                    value: key("bonus"),
                },
            )],
        });
    assert!(f.queries.requests.is_empty());
    let report = evaluate(&f);
    assert!(
        !report
            .effects
            .iter()
            .any(|effect| effect.key.invocation.owner == skill_owner),
        "listing a possible skill must not instantiate its Actor programs"
    );
    for use_id in [30, 31] {
        assert!(
            report
                .gaps
                .iter()
                .any(|gap| gap.provider == Some(summoner_provider(use_id))
                    && gap.subject == Some(skill_owner.clone())
                    && gap.reason == PlanGapReason::UnresolvedActivation),
            "expected the exact potential provider activation gap"
        );
    }
    let total = stat(ConcreteEntity::Actor(ActorKey::Player), "actor-total");
    assert_withheld(&report, &total);
    // Independently declared child projections remain exact component evidence;
    // report-level closure still withholds their final published values.
    for (use_id, level) in [(30, 11), (31, 20)] {
        let target = stat(ConcreteEntity::Actor(child_actor(use_id)), "child-level");
        let projections = report
            .effects
            .iter()
            .filter(|effect| {
                effect.target
                    == BoundEffectTarget::Value {
                        key: target.clone(),
                    }
                    && effect.key.invocation.owner == summoner_owner()
                    && effect.key.invocation.program == key("supply-child")
                    && effect.key.effect == key("project")
                    && effect.key.invocation.origin
                        == RuleOrigin::Provider {
                            provider: summoner_provider(use_id),
                        }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            projections.len(),
            1,
            "expected one exact parent projection for use {use_id}"
        );
        assert_eq!(
            projections[0].value,
            EffectValue::Known {
                value: integer(level)
            }
        );
        assert_withheld(&report, &target);
    }
}
#[test]
fn selected_support_action_cannot_bypass_the_unsupported_provider_relation() {
    let mut f = Fixture::new();
    f.add_action_route();
    f.routes.clear();
    f.schema.definitions.push(DefinitionDescriptor::Gem(known(
        def("support"),
        GemSchema {
            level: IntegerRange {
                minimum: BoundedInteger::new(1).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            },
            roles: vec![AuthoredGemRole::SupportAssignment],
            skills: DeclaredSet::complete(vec![def("skill")]),
            quality: QualityUseSchema {
                presence: QualityPresence::Forbidden,
                allowed_kinds: empty(),
            },
            declarations: declarations(),
        },
    )));
    let DefinitionDescriptor::Metric(metric) = f
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == def::<MetricDefinition>("requested").address())
        .unwrap()
    else {
        panic!()
    };
    let SchemaState::Known(metric) = &mut metric.schema else {
        panic!()
    };
    metric.provider_roles.push(ProviderRole::SupportAssignment);
    f.build.gems.push(GemInstance {
        id: occurrence(51),
        definition: def("support"),
        parameters: vec![],
        level: 10,
        quality: None,
    });
    f.build.supports.push(SupportAssignment {
        id: occurrence(50),
        support: occurrence(51),
        target: SkillTarget::Authored(occurrence(20)),
        enabled: true,
    });
    let provider = ProviderKey {
        root: ProviderRoot::SupportAssignment(occurrence(50)),
        grant_path: vec![],
    };
    let mut selected = action();
    selected.action.provider = provider.clone();
    f.queries.requests[0].target = MetricTarget::Action(Box::new(selected.clone()));
    f.owner_mut(&SchemaSubject::Slot(ActionOutputDefId::address(&output())))
        .programs
        .members
        .push(RuleProgram {
            id: key("must-stay-unsupported"),
            context: RuleEntityKind::Action,
            reads: vec![],
            nodes: vec![literal("constant", 777)],
            effects: vec![derive(
                "constant",
                RuleEntity::Current,
                "routed",
                "constant",
            )],
        });
    let report = evaluate(&f);
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.provider == Some(provider.clone())
                && gap.reason == PlanGapReason::UnsupportedRelation),
        "expected the selected support provider's unsupported relation gap"
    );
    let effects = report
        .effects
        .iter()
        .filter(|effect| effect.key.invocation.program == key("must-stay-unsupported"))
        .collect::<Vec<_>>();
    assert_eq!(
        effects.len(),
        1,
        "the selected output is retained as unsupported evidence"
    );
    assert!(
        matches!(
            effects[0].value,
            EffectValue::Unresolved {
                reason: PlanGapReason::UnsupportedRelation,
                ..
            }
        ),
        "wrong selected action effect status: {:?}",
        effects[0].value
    );
    let output_key = stat(ConcreteEntity::Action(Box::new(selected)), "routed");
    assert_withheld(&report, &output_key);
}
