//! Current source selection data on the finite native Sniper graph.
use super::*;
use poe_optimizer_core::{owned_routing::*, owned_schema::*};

fn authored() -> ActionOutputRoutes {
    let mut routes: Vec<ActionOutputRoutes> = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/minion-attack-selection/routes.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(routes.len(), 1);
    routes.pop().unwrap()
}

fn output(i: &ReplayInput) -> &ActionOutputRoutes {
    i.routing
        .outputs
        .iter()
        .find(|r| r.output.slot == def(0x22))
        .unwrap()
}

fn check_decisions(i: &ReplayInput, report: &SupportEffectsReport) {
    let expected = authored();
    let selectors = expected.source_selectors.as_ref().unwrap();
    assert!(selectors.is_complete());
    assert_eq!(selectors.members.len(), 1);
    let id = &selectors.members[0].id;
    let decisions: Vec<_> = effects(report)
        .effects
        .iter()
        .filter(|e| matches!(&e.target, BoundEffectTarget::SourceSelection { selector, .. } if selector == id))
        .collect();
    assert_eq!(
        decisions.len(),
        2,
        "one decision per exact Basic Action, none for Gas"
    );
    let basics: Vec<_> = actions(i)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
        .collect();
    assert_eq!(basics.len(), 2);
    for action in basics {
        let found: Vec<_> = decisions.iter().filter(|e| matches!(&e.target,
            BoundEffectTarget::SourceSelection { action: actual, .. } if actual.as_ref() == action
        )).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].value,
            EffectValue::Known {
                value: ParameterValue::Boolean(true)
            }
        );
    }
}

#[test]
fn published_selection_is_shared_without_changing_any_numeric_value() {
    let i = load();
    let expected = authored();
    assert_eq!(output(&i).routes.members, expected.routes.members);
    assert_eq!(output(&i).source_selectors, expected.source_selectors);
    // The finite fixture closes its own route inventory; the published owner
    // retains every independent numerical and coverage gap.
    assert!(!expected.routes.is_complete());
    let selected = run(&i);
    check_decisions(&i, &selected);
    let mut direct = i.clone();
    let current = direct
        .routing
        .outputs
        .iter_mut()
        .find(|r| r.output == expected.output)
        .unwrap();
    let mut changed = 0;
    for route in &mut current.routes.members {
        if let ActionStatRouteSource::Selected { stats, .. } = &route.source {
            assert_eq!(stats.len(), 1);
            route.source = ActionStatRouteSource::ActionActor {
                stat: stats[0].stat.clone(),
            };
            changed += 1;
        }
    }
    assert_eq!(changed, 4);
    current.source_selectors = Some(DeclaredSet::complete(vec![]));
    direct.rebind_test_edit().unwrap();
    let inverse = run(&direct);
    assert!(
        effects(&selected).values == effects(&inverse).values,
        "replacing direct transport with the proved source choice must preserve all computed values"
    );
    assert!(
        !effects(&inverse)
            .effects
            .iter()
            .any(|e| matches!(e.target, BoundEffectTarget::SourceSelection { .. }))
    );
}

#[test]
fn chosen_source_missing_minimum_never_falls_back_and_workers_remain_isolated() {
    let a = load();
    let mut b = a.clone();
    b.build
        .gems
        .iter_mut()
        .find(|g| g.definition == def(0x11))
        .unwrap()
        .level -= 1;
    let mut missing = a.clone();
    let mut removed = 0;
    for owner in &mut missing.rules.owners {
        for program in &mut owner.programs.members {
            let before = program.effects.len();
            program.effects.retain(|e| {
                !matches!(&e.effect,
                    RuleEffectKind::Derive { stat, .. } if *stat == def(0x320f)
                )
            });
            removed += before - program.effects.len();
        }
    }
    assert_eq!(removed, 1, "only the intrinsic minimum producer");
    missing.rebind_test_edit().unwrap();
    let inputs = [&a, &b, &missing];
    let plans = inputs.map(|i| i.compile().unwrap());
    let fresh = plans
        .each_ref()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap());
    for (i, report) in inputs.into_iter().zip(&fresh) {
        check_decisions(i, report);
    }
    let basics: Vec<_> = actions(&b)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
        .collect();
    assert_eq!(basics.len(), 2);
    for action in &basics {
        assert!(matches!(
            action_value(&fresh[1], action, 0x3212),
            EffectValue::Known { .. }
        ));
    }
    assert_ne!(
        action_value(&fresh[1], basics[0], 0x3212),
        action_value(&fresh[1], basics[1], 0x3212)
    );
    for action in actions(&missing)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
    {
        assert!(matches!(
            action_value(&fresh[2], action, 0x3212),
            EffectValue::Unresolved { .. }
        ));
        assert!(matches!(
            action_value(&fresh[2], action, 0x3213),
            EffectValue::Known { .. }
        ));
    }
    let mut scratch = plans[0].new_scratch();
    for n in [0, 1, 2, 0] {
        assert!(plans[n].evaluate(&mut scratch).unwrap() == fresh[n]);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..12).into_par_iter().for_each_init(
                || plans[0].new_scratch(),
                |scratch, n| {
                    assert!(plans[n % 3].evaluate(scratch).unwrap() == fresh[n % 3]);
                },
            );
        });
}
