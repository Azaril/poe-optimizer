//! General application semantics using explicitly synthetic, finite providers.
//! These fixtures do not assert an Offering value or real-build completeness.
#[path = "support/application_group_queries.rs"]
mod checked_queries;
#[allow(dead_code)]
#[path = "support/owned_computed_support_fixture.rs"]
mod fixture;
#[path = "support/effect_application_followups.rs"]
mod followups;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_routing::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use std::sync::Arc;

fn known_entry<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn world() -> Fixture {
    let mut f = Fixture::new();
    f.add_generated_actors();
    for (id, ty) in [
        ("delivered", ComputedValueType::Integer),
        ("secondary", ComputedValueType::Integer),
        ("missing-strength", ComputedValueType::Integer),
        ("missing-active", ComputedValueType::Boolean),
    ] {
        f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(known_entry(
                def(id),
                StatSchema {
                    value: ty,
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
    }
    for id in ["delivered", "secondary"] {
        f.owner_mut(&class_owner())
            .programs
            .members
            .push(RuleProgram {
                id: key(&format!("aggregate-{id}")),
                context: RuleEntityKind::Actor,
                reads: vec![contributions("incoming", RuleEntity::Current, id)],
                nodes: vec![read_node("value", "incoming")],
                effects: vec![derive("total", RuleEntity::Current, id, "value")],
            });
    }
    f
}
fn application(id: &str) -> EffectApplicationRule {
    EffectApplicationRule {
        id: key(id),
        source: EffectApplicationSource::OwnedSlot { slot: child_slot() },
        targets: vec![EffectApplicationTarget::Player],
        activation: key("active"),
        program: RuleProgram {
            id: key("deliver"),
            context: RuleEntityKind::Actor,
            reads: vec![read(
                "strength",
                RuleReadSource::Stat {
                    entity: RuleEntity::EffectSource,
                    stat: def("child-level"),
                },
            )],
            nodes: vec![
                node(
                    "active",
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    },
                ),
                read_node("strength", "strength"),
                literal("hundred", 100),
                node(
                    "reverse",
                    RuleExpression::Subtract {
                        left: key("hundred"),
                        right: key("strength"),
                    },
                ),
            ],
            effects: vec![
                effect(
                    "damage",
                    RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: def("delivered"),
                        contribution: ContributionKind::Add,
                        value: key("strength"),
                    },
                ),
                effect(
                    "speed",
                    RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: def("secondary"),
                        contribution: ContributionKind::Add,
                        value: key("reverse"),
                    },
                ),
            ],
        },
        stacking: ["damage", "speed"]
            .into_iter()
            .map(|id| EffectStackingRule {
                effect: key(id),
                family: key("finite-buff"),
                modifier: key(id),
                reduction: EffectStackingReduction::Maximum,
            })
            .collect(),
    }
}
fn input(
    f: &Fixture,
    schema: &OwnedDefinitionSchemaPackage,
    rows: DeclaredSet<EffectApplicationRule>,
) -> RulePackageInput {
    RulePackageInput {
        support_discovery: Some(SupportDiscoveryInput {
            providers: f
                .owners
                .iter()
                .map(|row| SupportSourceDomainDeclaration {
                    owner: row.owner.clone(),
                    domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
                })
                .collect(),
        }),
        existing_actor_rules: None,
        contribution_queries: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("effect-application-tests"),
        semantics_version: key("finite-test"),
        operations_version: key(OWNED_RULE_OPERATIONS_V15),
        definitions: schema.identity().clone(),
        owners: f.owners.clone(),
        tables: f.tables.clone(),
        receivers: f.receivers.clone(),
        effect_applications: Some(rows),
    }
}
fn compile(
    f: &Fixture,
    rows: DeclaredSet<EffectApplicationRule>,
    limits: PlanLimits,
) -> std::result::Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>, String> {
    let schema = Arc::new(
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default())
            .map_err(|e| e.to_string())?,
    );
    let rules = Arc::new(
        CompiledRulePackage::compile(
            &input(f, &schema, rows),
            schema.as_ref(),
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let routing = Arc::new(
        OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("empty-routing"),
                definitions: schema.identity().clone(),
                outputs: f.routes.clone(),
            },
            schema.as_ref(),
            Default::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    OwnedEffectPlan::compile(Arc::new(f.request()), schema, rules, routing, limits)
        .map_err(|e| e.to_string())
}
fn plan(
    f: &Fixture,
    rows: Vec<EffectApplicationRule>,
) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
    compile(f, DeclaredSet::complete(rows), Default::default()).unwrap()
}
fn evaluate(f: &Fixture, rows: Vec<EffectApplicationRule>) -> OwnedEffectsReport {
    let plan = plan(f, rows);
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn value<'a>(report: &'a OwnedEffectsReport, id: &str) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(id),
                }
        })
        .unwrap()
        .value
}
fn number(report: &OwnedEffectsReport, id: &str) -> i64 {
    let EffectValue::Known {
        value: ParameterValue::Integer(value),
    } = value(report, id)
    else {
        panic!("expected known {id}: {report:?}")
    };
    value.get()
}

#[test]
fn maxima_are_per_modifier_with_exact_source_provenance_and_no_queries() {
    let f = world();
    assert!(f.queries.requests.is_empty());
    let report = evaluate(&f, vec![application("a")]);
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(
        (number(&report, "delivered"), number(&report, "secondary")),
        (20, 89)
    );
    assert_eq!(report.application_groups.len(), 2);
    for group in &report.application_groups {
        assert_eq!(group.candidates.len(), 2);
        assert_eq!(group.co_winners.len(), 1);
        let expected = if group.key.effect == key("damage") {
            child_actor(31)
        } else {
            child_actor(30)
        };
        assert!(
            matches!(&group.co_winners[0].invocation.origin, RuleOrigin::EffectApplication { source: ConcreteEntity::Actor(actor), recipient: ConcreteEntity::Actor(ActorKey::Player), .. } if *actor == expected)
        );
    }
    assert_eq!(
        report
            .effects
            .iter()
            .filter(|row| matches!(row.target, BoundEffectTarget::ApplicationCandidate { .. }))
            .count(),
        4
    );
    assert_eq!(
        report
            .effects
            .iter()
            .filter(|row| matches!(
                row.key.invocation.origin,
                RuleOrigin::EffectApplicationGroup { .. }
            ) && matches!(row.target, BoundEffectTarget::Contribution { .. }))
            .count(),
        2
    );
}

#[test]
fn false_activation_is_lazy_but_unknown_activation_blocks_every_candidate() {
    let f = world();
    let mut row = application("inactive");
    row.program.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::EffectSource,
        stat: def("missing-strength"),
    };
    row.program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("active"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    let report = evaluate(&f, vec![row.clone()]);
    assert!(report.gaps.is_empty());
    assert_eq!(number(&report, "delivered"), 0);
    assert!(
        report
            .application_groups
            .iter()
            .all(|group| group.value == EffectValue::Inactive && group.co_winners.is_empty())
    );
    assert!(
        report
            .effects
            .iter()
            .filter(|e| matches!(e.target, BoundEffectTarget::ApplicationCandidate { .. }))
            .all(|e| e.value == EffectValue::Inactive)
    );
    row.program.reads.push(RuleRead {
        id: key("is-active"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Stat {
            entity: RuleEntity::EffectSource,
            stat: def("missing-active"),
        },
    });
    row.program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("active"))
        .unwrap()
        .expression = RuleExpression::Read {
        input: key("is-active"),
    };
    let report = evaluate(&f, vec![row]);
    assert!(matches!(
        value(&report, "delivered"),
        EffectValue::Unresolved { .. }
    ));
    assert!(report.application_groups.iter().all(|group| matches!(
        group.value,
        EffectValue::Unresolved { .. }
    ) && group.co_winners.is_empty()));
}

#[test]
fn negative_maxima_are_not_zero_seeded_and_all_equal_winners_are_preserved() {
    let f = world();
    let mut rows = vec![application("first"), application("second")];
    for (row, amount) in rows.iter_mut().zip([-8, -3]) {
        row.program
            .nodes
            .iter_mut()
            .find(|n| n.id == key("strength"))
            .unwrap()
            .expression = RuleExpression::Literal {
            value: integer(amount),
        };
    }
    let report = evaluate(&f, rows);
    assert_eq!(number(&report, "delivered"), -3);
    let group = report
        .application_groups
        .iter()
        .find(|g| g.key.effect == key("damage"))
        .unwrap();
    assert_eq!(group.candidates.len(), 4);
    assert_eq!(group.co_winners.len(), 2);
    assert!(group.co_winners.iter().all(|winner|matches!(&winner.invocation.origin,RuleOrigin::EffectApplication { application, .. } if *application == key("second"))));
}

#[test]
fn source_reads_never_fall_back_to_recipient_and_raw_ancestor_reads_reject() {
    let mut f = world();
    f.owner_mut(&class_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("recipient-only"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![literal("strength", 999)],
            effects: vec![derive(
                "set",
                RuleEntity::Current,
                "missing-strength",
                "strength",
            )],
        });
    let mut row = application("missing-source");
    row.program.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::EffectSource,
        stat: def("missing-strength"),
    };
    let report = evaluate(&f, vec![row.clone()]);
    assert!(matches!(
        value(&report, "delivered"),
        EffectValue::Unresolved { .. }
    ));
    row.program.reads[0].source = RuleReadSource::GemLevel;
    assert!(
        compile(&f, DeclaredSet::complete(vec![row]), Default::default())
            .err()
            .unwrap()
            .contains("raw input authority")
    );
}

#[test]
fn inactive_sources_and_partial_candidate_membership_remain_distinct() {
    let mut f = world();
    for gem in &mut f.build.gems {
        gem.parameters[0].value = ParameterValue::Boolean(false);
    }
    let report = evaluate(&f, vec![application("a")]);
    assert_eq!(number(&report, "delivered"), 0);
    assert!(
        report
            .application_groups
            .iter()
            .all(|group| group.co_winners.is_empty())
    );
    let rows = DeclaredSet {
        members: vec![application("a")],
        closure: SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: summoner_owner(),
                facet: SchemaFacet::GameRules,
                code: key("unknown-applications"),
            }],
        },
    };
    let p = compile(&f, rows, Default::default()).unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialEffectApplications)
    );
    assert!(matches!(
        value(&report, "delivered"),
        EffectValue::Unresolved { .. }
    ));
}

#[test]
fn different_recipients_do_not_share_group_state_and_pairs_are_bounded() {
    let f = world();
    let mut row = application("a");
    row.targets
        .push(EffectApplicationTarget::OwnedSlot { slot: child_slot() });
    let report = evaluate(&f, vec![row.clone()]);
    assert_eq!(report.application_groups.len(), 6);
    assert!(
        report
            .application_groups
            .iter()
            .all(|group| group.candidates.len() == 2)
    );
    let limit = PlanLimits {
        max_invocations: 5,
        ..Default::default()
    };
    assert!(
        compile(&f, DeclaredSet::complete(vec![row]), limit)
            .err()
            .unwrap()
            .contains("application pairs")
    );
}

#[test]
fn scaling_precedes_group_selection_and_existing_guards_are_conjoined() {
    let f = world();
    let mut a = application("a");
    a.program.nodes.push(literal("factor", 3));
    a.program.nodes.push(node(
        "scaled",
        RuleExpression::ScaleInteger {
            value: key("strength"),
            count: key("factor"),
        },
    ));
    if let RuleEffectKind::Contribute { value, .. } = &mut a.program.effects[0].effect {
        *value = key("scaled");
    }
    let mut b = application("b");
    b.program.nodes.push(node(
        "original-guard",
        RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    ));
    b.program.effects[0].when = Some(key("original-guard"));
    b.program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("strength"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: integer(1000),
    };
    let report = evaluate(&f, vec![a, b]);
    assert_eq!(number(&report, "delivered"), 60);
}

#[test]
fn graph_cycles_and_inconsistent_modifier_channels_reject() {
    let f = world();
    let mut row = application("cycle");
    row.program.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def("delivered"),
    };
    assert!(
        compile(&f, DeclaredSet::complete(vec![row]), Default::default())
            .err()
            .unwrap()
            .contains("cycle")
    );
    let a = application("a");
    let mut b = application("b");
    if let RuleEffectKind::Contribute { stat, .. } = &mut b.program.effects[0].effect {
        *stat = def("secondary");
    }
    assert!(
        compile(&f, DeclaredSet::complete(vec![a, b]), Default::default())
            .err()
            .unwrap()
            .contains("modifier group changes")
    );
}

#[test]
fn scratch_reuse_and_rayon_workers_preserve_application_provenance() {
    let f = world();
    let p = Arc::new(plan(&f, vec![application("a")]));
    let mut scratch = p.new_scratch();
    let expected = p.evaluate(&mut scratch).unwrap();
    let mut other = world();
    other.build.gems[0].level = 70;
    let q = plan(&other, vec![application("a")]);
    q.evaluate(&mut scratch).unwrap();
    assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    let reports: Vec<_> = (0..24)
        .into_par_iter()
        .map_init(
            || p.new_scratch(),
            |scratch, _| p.evaluate(scratch).unwrap(),
        )
        .collect();
    assert!(reports.iter().all(|report| report == &expected));
}

#[test]
fn explicit_application_registry_is_versioned_and_legacy_bytes_omit_it() {
    let f = world();
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut rules = input(&f, &schema, DeclaredSet::complete(vec![]));
    rules.effect_applications = None;
    assert!(CompiledRulePackage::compile(&rules, &schema, Default::default()).is_err());
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V14);
    assert!(
        !serde_json::to_value(&rules)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("effect_applications")
    );
    CompiledRulePackage::compile(&rules, &schema, Default::default()).unwrap();
    rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    assert!(CompiledRulePackage::compile(&rules, &schema, Default::default()).is_err());
}
