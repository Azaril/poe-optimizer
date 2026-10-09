//! The intrinsic source and checked combined factor on the joined native replay.
use super::*;
use poe_optimizer_core::owned_schema::*;

const PROFILE: &str = "intrinsic-added-attack-percentage";
const ACTION: &str = "intrinsic-added-attack-factor";
const COMBINED: &str = "combined-added-attack-factor";
const QUERY: &str = "intrinsic-added-attack-factors";
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn source(i: &mut ReplayInput) -> &mut RuleProgram {
    i.rules
        .owners
        .iter_mut()
        .flat_map(|o| &mut o.programs.members)
        .find(|p| p.id == key("finite-actor-baseline"))
        .unwrap()
}
fn profile(i: &mut ReplayInput, scale: Option<f64>) {
    let p = source(i);
    let effect = p
        .effects
        .iter()
        .position(|e| {
            matches!(&e.effect,
                RuleEffectKind::Derive { stat, .. } if *stat == def(0x2537)
            )
        })
        .unwrap();
    if let Some(scale) = scale {
        let RuleEffectKind::Derive { value, .. } = &p.effects[effect].effect else {
            panic!()
        };
        let node = p.nodes.iter_mut().find(|n| &n.id == value).unwrap();
        assert!(matches!(node.expression, RuleExpression::Literal { .. }));
        node.expression = RuleExpression::Literal {
            value: ParameterValue::Quantity(FiniteQuantity::new(scale, def(1)).unwrap()),
        };
    } else {
        p.effects.remove(effect);
    }
    i.rebind_test_edit().unwrap();
}
fn contributions(r: &SupportEffectsReport) -> Vec<&BoundEffectResult> {
    effects(r)
        .effects
        .iter()
        .filter(|e| {
            matches!(&e.target,
                BoundEffectTarget::Contribution { key } if key.stat == def(0x336d)
            )
        })
        .collect()
}
fn no_known_value(r: &SupportEffectsReport, entity: ConcreteEntity, stat: u64) {
    let key = PlanValueKey::Stat {
        entity,
        stat: def(stat),
    };
    let row = effects(r).values.iter().find(|v| v.key == key);
    // A channel with no producer need not have a materialized value row.
    assert!(row.is_none_or(|v| matches!(v.value, EffectValue::Unresolved { .. })));
}
fn check_combined(i: &ReplayInput, r: &SupportEffectsReport, expected: Option<f64>) {
    let mut basics = 0;
    for action in actions(i) {
        if action.action.output.slot == def(0x22) {
            basics += 1;
            match expected {
                Some(expected) => assert_eq!(
                    action_value(r, action, 0x336e),
                    &EffectValue::Known {
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(expected, def(1)).unwrap()
                        )
                    }
                ),
                None => no_known_value(r, ConcreteEntity::Action(Box::new(action.clone())), 0x336e),
            }
        } else {
            assert!(!effects(r).values.iter().any(|v| v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Action(Box::new(action.clone())),
                    stat: def(0x336e),
                }));
        }
    }
    assert_eq!(basics, 2);
}
fn check(i: &ReplayInput, r: &SupportEffectsReport, scale: Option<f64>) {
    let rows = contributions(r);
    assert_eq!(
        rows.len(),
        2,
        "one source per Basic Action, no Gas contribution"
    );
    let basic: Vec<_> = actions(i)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
        .collect();
    assert_eq!(basic.len(), 2, "both exact Basic recipients are checked");
    for a in basic {
        let row: Vec<_> = rows.iter().filter(|e| matches!(&e.target,
            BoundEffectTarget::Contribution { key } if key.entity == ConcreteEntity::Action(Box::new(a.clone()))
                && key.kind == ContributionKind::Multiply
        )).collect();
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].key.invocation.program, key(ACTION));
        assert_eq!(
            row[0].key.invocation.entity,
            ConcreteEntity::Action(Box::new(a.clone()))
        );
        let actor = value(r, &a.action.actor, 0x336b);
        let routed = action_value(r, a, 0x336c);
        if let Some(scale) = scale {
            let raw = (scale - 1.) * 100.;
            let expected = EffectValue::Known {
                value: ParameterValue::Quantity(FiniteQuantity::new(raw, def(2)).unwrap()),
            };
            assert_eq!(*actor, expected);
            assert_eq!(*routed, expected);
            assert_eq!(
                row[0].value,
                if raw == 0. {
                    EffectValue::Inactive
                } else {
                    EffectValue::Known {
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(1. + raw / 100., def(1)).unwrap(),
                        ),
                    }
                }
            );
        } else {
            assert!(matches!(actor, EffectValue::Unresolved { .. }));
            assert!(matches!(routed, EffectValue::Unresolved { .. }));
            assert!(matches!(row[0].value, EffectValue::Unresolved { .. }));
        }
    }
}

#[test]
fn authored_intrinsic_source_is_preserved_on_exact_basic_actions() {
    let i = load();
    let packet = root().join("data/owned/poe2/3887ae68/intrinsic-added-attack-damage");
    let consumer: Value =
        serde_json::from_slice(&fs::read(packet.join("consumer.json")).unwrap()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(consumer["owners"].clone()).unwrap();
    assert_eq!(owners.len(), 2);
    for owner in owners {
        let actual = i
            .rules
            .owners
            .iter()
            .find(|o| o.owner == owner.owner)
            .unwrap();
        for program in owner.programs.members {
            assert!(actual.programs.members.contains(&program));
        }
    }
    let routes: Vec<poe_optimizer_core::owned_routing::ActionOutputRoutes> =
        serde_json::from_slice(&fs::read(packet.join("routes.json")).unwrap()).unwrap();
    assert_eq!(routes.len(), 1);
    for route in routes {
        let actual = i
            .routing
            .outputs
            .iter()
            .find(|r| r.output == route.output)
            .unwrap();
        for member in route.routes.members {
            assert!(actual.routes.members.contains(&member));
        }
    }
    let r = run(&i);
    check(&i, &r, Some(1.15));
    check_combined(&i, &r, Some(1.15));
    let combined = root().join("data/owned/poe2/3887ae68/combined-added-attack-damage");
    let consumer: Value =
        serde_json::from_slice(&fs::read(combined.join("consumer.json")).unwrap()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(consumer["owners"].clone()).unwrap();
    for owner in owners {
        let actual = i
            .rules
            .owners
            .iter()
            .find(|o| o.owner == owner.owner)
            .unwrap();
        for program in owner.programs.members {
            assert!(actual.programs.members.contains(&program));
        }
    }
    let queries: Vec<ContributionQuery> =
        serde_json::from_slice(&fs::read(combined.join("queries.json")).unwrap()).unwrap();
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].id, key(QUERY));
    for query in queries {
        assert!(
            i.rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&query)
        );
    }
    let mut reordered = i.clone();
    reordered.build.skills.reverse();
    reordered.build.gems.reverse();
    assert_eq!(run(&reordered), r);
}

#[test]
fn injected_profile_unknown_identity_and_changes_survive_reused_workers() {
    let original = load();
    let mut identity = original.clone();
    profile(&mut identity, Some(1.));
    let mut changed = original.clone();
    profile(&mut changed, Some(1.23456789));
    let mut missing = original.clone();
    profile(&mut missing, None);
    let mut zero = original.clone();
    profile(&mut zero, Some(0.));
    let inputs = [&original, &identity, &changed, &missing, &zero];
    let plans = inputs.map(|i| i.compile().unwrap());
    let fresh = plans
        .each_ref()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap());
    for ((i, r), scale) in
        inputs
            .into_iter()
            .zip(&fresh)
            .zip([Some(1.15), Some(1.), Some(1.23456789), None, Some(0.)])
    {
        check(i, r, scale);
    }
    // The altered profiles are finite arithmetic controls, not additional
    // production source domains admitted by the authored packet.
    for ((i, r), expected) in
        inputs
            .into_iter()
            .zip(&fresh)
            .zip([Some(1.15), Some(1.), Some(1.23), None, Some(0.)])
    {
        check_combined(i, r, expected);
    }
    let mut scratch = plans[0].new_scratch();
    for index in [0, 1, 2, 3, 4, 0] {
        assert_eq!(plans[index].evaluate(&mut scratch).unwrap(), fresh[index]);
    }
    let reports = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..20)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |s, n| plans[n % 5].evaluate(s).unwrap(),
                )
                .collect::<Vec<_>>()
        });
    for (n, report) in reports.into_iter().enumerate() {
        assert_eq!(report, fresh[n % 5]);
    }
}

#[test]
fn profile_data_alone_does_not_authorize_the_intrinsic_source() {
    let mut i = load();
    let owner = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.programs.members.iter().any(|p| p.id == key(PROFILE)))
        .unwrap();
    let source_owner = owner.owner.clone();
    owner.programs.members.retain(|p| p.id != key(PROFILE));
    i.stages
        .programs
        .members
        .retain(|p| !(p.owner == source_owner && p.program == key(PROFILE)));
    i.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .retain(|p| !(p.owner == source_owner && p.program == key(PROFILE)));
    i.rebind_test_edit().unwrap();
    let r = run(&i);
    check_combined(&i, &r, None);
    for actor in actors(&i) {
        assert!(
            matches!(value(&r, &actor, 0x2537), EffectValue::Known { .. }),
            "generic profile remains available"
        );
    }
    // Removing the authorized producer cannot use a fallback factor despite
    // the same generic profile. This is not a foreign Actor admission fixture.
    for a in actions(&i)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
    {
        no_known_value(&r, ConcreteEntity::Actor(a.action.actor.clone()), 0x336b);
        no_known_value(&r, ConcreteEntity::Action(Box::new(a.clone())), 0x336c);
    }
    let rows = contributions(&r);
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|r| matches!(r.value, EffectValue::Unresolved { .. }))
    );
}

#[test]
fn actor_amount_needs_its_exact_action_route() {
    let mut i = load();
    let route = i
        .routing
        .outputs
        .iter_mut()
        .find(|r| r.output.slot == def(0x22))
        .unwrap();
    let count = route.routes.members.len();
    route.routes.members.retain(|r| r.id != key(PROFILE));
    assert_eq!(route.routes.members.len() + 1, count);
    i.rebind_test_edit().unwrap();
    let r = run(&i);
    check_combined(&i, &r, None);
    for a in actions(&i)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
    {
        assert!(matches!(
            value(&r, &a.action.actor, 0x336b),
            EffectValue::Known { .. }
        ));
        no_known_value(&r, ConcreteEntity::Action(Box::new(a.clone())), 0x336c);
    }
    let rows = contributions(&r);
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|r| matches!(r.value, EffectValue::Unresolved { .. }))
    );
}

#[test]
fn intrinsic_source_keeps_partial_coverage_and_stage_dependencies() {
    let original = load();
    for (program, stage) in [
        (PROFILE, "prepare"),
        (ACTION, "facts"),
        (COMBINED, "intrinsic-added-damage-profile"),
    ] {
        let mut early = original.clone();
        early
            .stages
            .programs
            .members
            .iter_mut()
            .find(|p| p.program == key(program))
            .unwrap()
            .stage = key(stage);
        let error = early
            .rebind_test_edit()
            .expect_err("reads must follow both the profile producer and its Action route");
        assert!(
            error.contains("frozen channel read occurs before or outside frozen stage"),
            "{error}"
        );
    }
    let mut partial = original.clone();
    let owner = partial
        .rules
        .owners
        .iter_mut()
        .find(|o| o.programs.members.iter().any(|p| p.id == key(PROFILE)))
        .unwrap();
    owner.programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("unreviewed-intrinsic-source"),
        }],
    };
    partial.rebind_test_edit().unwrap();
    let p = partial.compile().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}

#[test]
fn combined_factor_requires_complete_exact_contributor_membership() {
    let original = load();
    for (amount, active) in [(0., true), (1., false), (1., true)] {
        let mut i = original.clone();
        let p = i
            .rules
            .owners
            .iter_mut()
            .flat_map(|o| &mut o.programs.members)
            .find(|p| p.id == key(ACTION))
            .unwrap();
        p.nodes.extend([
            RuleNode {
                id: key("unlisted-value"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Quantity(FiniteQuantity::new(amount, def(1)).unwrap()),
                },
            },
            RuleNode {
                id: key("unlisted-active"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(active),
                },
            },
        ]);
        p.effects.push(RuleEffect {
            id: key("unlisted-factor"),
            when: Some(key("unlisted-active")),
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def(0x336d),
                contribution: ContributionKind::Multiply,
                value: key("unlisted-value"),
            },
        });
        let error = i
            .rebind_test_edit()
            .expect_err("potential contributors need exact membership");
        assert!(error.contains("no declared membership"), "{error}");
    }
    let mut partial = original.clone();
    let query = partial
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == key(QUERY))
        .unwrap();
    query.groups[0].members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(DefinitionAddress::Stat(query.stat.clone())),
            facet: SchemaFacet::GameRules,
            code: key("unreviewed-added-damage-supplier"),
        }],
    };
    partial.rebind_test_edit().unwrap();
    assert!(matches!(
        run(&partial).outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}
